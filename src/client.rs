use std::{sync::{Arc, Mutex, RwLock}, ops::Deref, collections::HashMap};

use actix_web::{error::ErrorUnauthorized, FromRequest, web::Data};
use log::warn;
use mediasoup::{webrtc_transport::WebRtcTransport, producer::{Producer, ProducerId}, prelude::{Consumer, ConsumerId}, transport::TransportId};
use nanoid::nanoid;

use crate::{channel::Channel, util::constant_time_compare, Clients, MutexMap};

#[derive(Debug)]
pub struct Client {
    pub identity: String,
    pub token: String,
    pub channel: Arc<Channel>,
    // c2s
    pub c2s_transports: RwLock<HashMap<String, WebRtcTransport>>,
    pub producers: MutexMap<Producer>,
    // s2c
    pub s2c_transports: RwLock<HashMap<String, WebRtcTransport>>,
    pub consumers: MutexMap<Consumer>,
}

impl Client {
    pub fn new_random(channel: Arc<Channel>) -> Self {
        Self {
            identity: nanoid!(),
            token: nanoid!(64),
            channel,
            c2s_transports: RwLock::new(HashMap::new()),
            producers: Mutex::new(HashMap::new()),
            s2c_transports: RwLock::new(HashMap::new()),
            consumers: Mutex::new(HashMap::new()),
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
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        // extract the RTC-Identity header
        let rtc_identity = req
            .headers()
            .get("RTC-Identity")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // extract the RTC-Token header
        let rtc_token = req
            .headers()
            .get("RTC-Token")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        if rtc_token == None || rtc_identity == None {
            warn!("no token or identity");
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
            warn!("unknown identity: {}", rtc_identity);
            return std::future::ready(Err(ErrorUnauthorized("access_denied")));
        }

        let client = client.unwrap();

        if !constant_time_compare(&client.token, &rtc_token) {
            warn!("token mismatch for {:?}: expected {:?}, got {:?}", client.identity, client.token, rtc_token);
            return std::future::ready(Err(ErrorUnauthorized("access_denied")));
        }

        return std::future::ready(Ok(ClientEx(client)));
    }
}
