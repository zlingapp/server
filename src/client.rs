use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
};

use actix_rt::task::JoinHandle;
use actix_web::{
    error::{ErrorBadRequest, ErrorUnauthorized},
    web::{Data, Query},
    FromRequest,
};
use log::warn;
use mediasoup::{prelude::Consumer, producer::Producer, webrtc_transport::WebRtcTransport};
use nanoid::nanoid;
use serde::Deserialize;

use crate::{channel::Channel, util::constant_time_compare, Clients, MutexMap};

pub struct Client {
    pub identity: String,
    pub token: String,
    pub channel: Arc<Channel>,
    // c2s
    pub c2s_transport: RwLock<Option<WebRtcTransport>>,
    pub producers: MutexMap<Producer>,
    // s2c
    pub s2c_transport: RwLock<Option<WebRtcTransport>>,
    pub consumers: MutexMap<Consumer>,

    pub socket_session: RwLock<Option<actix_ws::Session>>,
    pub socket_watchdog_handle: Mutex<Option<JoinHandle<()>>>,

    pub last_ping: RwLock<Option<std::time::Instant>>,
}

impl Client {
    pub fn new_random(channel: Arc<Channel>) -> Self {
        Self {
            identity: nanoid!(),
            token: nanoid!(64),
            channel,
            c2s_transport: RwLock::new(None),
            producers: Mutex::new(HashMap::new()),
            s2c_transport: RwLock::new(None),
            consumers: Mutex::new(HashMap::new()),
            socket_session: RwLock::new(None),
            socket_watchdog_handle: Mutex::new(None),
            last_ping: RwLock::new(None),
        }
    }

    pub async fn cleanup(&self) {
        if let Some(watchdog) = self.socket_watchdog_handle.lock().unwrap().take() {
            // we connected successfully, so stop the watchdog now
            watchdog.abort();
        }
        if let Some(session) = self.socket_session.write().unwrap().take() {
            // we connected successfully, so close the session now
            session.close(None).await;
        }
    }
}

// extract the client from the request
pub struct ClientEx(pub Arc<Client>);

impl Deref for ClientEx {
    type Target = Arc<Client>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for ClientEx {
    type Error = actix_web::Error;

    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let trying_to_connect_to_ws = req.path() == "/ws" && req.method() == "GET";

        let rtc_identity;
        let rtc_token;

        if trying_to_connect_to_ws {
            // rtc identity and token are in the query string for ws connections
            // this is because the RTC-Identity and RTC-Token headers can't be set
            // because WebSocket() in the browser doesn't allow setting request options! :D

            #[derive(Deserialize)]
            struct IdAndToken {
                #[serde(rename = "i")]
                rtc_identity: String,
                #[serde(rename = "t")]
                rtc_token: String,
            }
            let query = Query::<IdAndToken>::from_query(req.query_string());

            match query {
                Ok(q) => {
                    rtc_identity = Some(q.rtc_identity.clone());
                    rtc_token = Some(q.rtc_token.clone());
                }
                Err(_) => {
                    return std::future::ready(Err(ErrorUnauthorized("access_denied")));
                }
            }
        } else {
            // extract the RTC-Identity header
            rtc_identity = req
                .headers()
                .get("RTC-Identity")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // extract the RTC-Token header
            rtc_token = req
                .headers()
                .get("RTC-Token")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
        }

        if rtc_token == None || rtc_identity == None {
            warn!(
                "no token and/or identity provided, denying access to {}",
                req.path()
            );
            return std::future::ready(Err(ErrorUnauthorized("access_denied")));
        }

        // SAFETY: this is fine because of the
        let rtc_identity = rtc_identity.unwrap();
        let rtc_token = rtc_token.unwrap();

        // get the client with that identity
        let client = req
            .app_data::<Data<Clients>>()
            .unwrap()
            .lock()
            .unwrap()
            .get(&rtc_identity)
            .cloned();

        if client.is_none() {
            warn!(
                "unknown identity {:?}, denying access to {}",
                rtc_identity,
                req.path()
            );
            return std::future::ready(Err(ErrorUnauthorized("access_denied")));
        }

        let client = client.unwrap();

        if !constant_time_compare(&client.token, &rtc_token) {
            warn!(
                "token mismatch for {:?}, denying access to {}",
                client.identity,
                req.path()
            );
            return std::future::ready(Err(ErrorUnauthorized("access_denied")));
        }

        if !trying_to_connect_to_ws && client.socket_session.read().unwrap().is_none() {
            warn!(
                "no socket session for {:?}, denying access to {}",
                client.identity,
                req.path()
            );
            return std::future::ready(Err(ErrorBadRequest("event_socket_not_connected")));
        }

        return std::future::ready(Ok(ClientEx(client)));
    }
}
