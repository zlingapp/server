use actix_rt::task::JoinHandle;
use actix_web::{
    web::{Data, Query},
    FromRequest,
};
use futures::Future;
use log::info;
use mediasoup::{prelude::Consumer, producer::Producer, webrtc_transport::WebRtcTransport};
use nanoid::nanoid;
use std::{
    collections::HashMap,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::RwLock;

use crate::{
    auth::{access_token::AccessToken, user::User},
    error::{macros::err, HResult, HandlerError, IntoHandlerErrorResult},
    voice::{channel::VoiceChannel, MutexMap, VoiceClients},
};
use crate::{realtime::socket::Socket, util::constant_time_compare};

use super::routes::voice_events::IdAndToken;

pub struct VoiceClient {
    pub identity: String,
    pub token: String,
    pub channel: Arc<VoiceChannel>,
    // c2s
    pub c2s_transport: RwLock<Option<WebRtcTransport>>,
    pub producers: MutexMap<Producer>,
    // s2c
    pub s2c_transport: RwLock<Option<WebRtcTransport>>,
    pub consumers: MutexMap<Consumer>,

    pub socket: tokio::sync::RwLock<Option<Arc<Socket>>>,
    // this is used to cancel the initial connect watch task
    pub socket_initial_connect_watch_handle: Mutex<Option<JoinHandle<()>>>,
    // the user that this client belongs to
    pub user: User,
}

impl VoiceClient {
    pub fn with_channel_and_user(channel: Arc<VoiceChannel>, user: User) -> Self {
        Self {
            identity: nanoid!(),
            token: nanoid!(64),
            channel,
            c2s_transport: None.into(),
            producers: HashMap::new().into(),
            s2c_transport: None.into(),
            consumers: HashMap::new().into(),
            socket: None.into(),
            socket_initial_connect_watch_handle: Mutex::new(None),
            user,
        }
    }

    pub fn cleanup(&self) {
        if let Some(handle) = self
            .socket_initial_connect_watch_handle
            .lock()
            .unwrap()
            .take()
        {
            handle.abort();
        }
    }
}

impl Drop for VoiceClient {
    fn drop(&mut self) {
        info!("client[{}]: dropped", self.identity);
    }
}

// extract the client from the request
pub struct VoiceClientEx(pub Arc<VoiceClient>);

impl Deref for VoiceClientEx {
    type Target = Arc<VoiceClient>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for VoiceClientEx {
    type Error = HandlerError;
    type Future = Pin<Box<dyn Future<Output = HResult<Self>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        // validate session
        let req = req.clone();

        Box::pin(async move {
            let trying_to_connect_to_ws = req.path() == "/voice/ws" && req.method() == "GET";

            if !trying_to_connect_to_ws {
                // validate session
                AccessToken::from_request(&req, &mut actix_web::dev::Payload::None).await?;

                // todo: a bunch of logic & checks here to make sure the user is allowed to do operations in the channel
                //       use the return value of the above line to get the user
            }

            let rtc_identity;
            let rtc_token;

            if trying_to_connect_to_ws {
                // rtc identity and token are in the query string for ws connections
                // this is because the RTC-Identity and RTC-Token headers can't be set
                // because WebSocket() in the browser doesn't allow setting request options! :D
                let query = Query::<IdAndToken>::from_query(req.query_string());

                match query {
                    Ok(q) => {
                        rtc_identity = Some(q.i.clone());
                        rtc_token = Some(q.t.clone());
                    }
                    Err(_) => {
                        // trying to connect to ws but no credentials in query
                        return err!(401);
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

            if rtc_token.is_none() || rtc_identity.is_none() {
                return err!(401);
            }

            // SAFETY: this is fine because of the check above
            let rtc_identity = rtc_identity.unwrap();
            let rtc_token = rtc_token.unwrap();

            // get the client with that identity
            let client = req
                .app_data::<Data<VoiceClients>>()
                .or_err(500)?
                .lock()
                .or_err(500)?
                .get(&rtc_identity)
                .cloned();

            if client.is_none() {
                return err!(403)?;
            }

            let client = client.unwrap();

            if !constant_time_compare(&client.token, &rtc_token) {
                return err!(403)?;
            }

            if !trying_to_connect_to_ws && client.socket.read().await.is_none() {
                return err!(400, "You need to connect to the voice event socket first.")?;
            }

            Ok(VoiceClientEx(client))
        })
    }
}
