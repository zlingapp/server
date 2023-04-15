use std::sync::Arc;

use actix_web::{
    error, get,
    web::{Data, Json, Query},
};
use derive_more::{Display, Error};

use mediasoup::{rtp_parameters::RtpCapabilitiesFinalized, worker_manager::WorkerManager};
use serde::{Deserialize, Serialize};

use crate::{channel::create_channel, client::Client, Channels, Clients};

#[derive(Deserialize)]
pub struct JoinVcQuery {
    // channel id
    c: String,
}

#[derive(Debug, Serialize)]
pub struct JoinVcReply {
    channel_id: String,
    identity: String,
    token: String,
    rtp: RtpCapabilitiesFinalized,
}

#[derive(Debug, Display, Error)]
pub struct JoinVcError {}

impl error::ResponseError for JoinVcError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::CONFLICT
    }
}

pub type JoinVcResponse = Result<Json<JoinVcReply>, JoinVcError>;

#[get("/join")]
pub async fn join_vc(clients: Data<Clients>, channels: Data<Channels>, wm: Data<WorkerManager>, query: Query<JoinVcQuery>) -> JoinVcResponse {
    // get the channel
    let channel = channels.lock().unwrap().get(&query.c).cloned();
    // channels lock is released here

    let channel = match channel {
        Some(existing) => existing.clone(),
        None => create_channel(&query.c, channels.clone(), wm.into_inner()).await,
    };

    // create a new client
    let client = Arc::new(Client::new_random(channel.clone()));

    // add the client to the channel's client list
    channel.clients.lock().unwrap().push(client.clone());

    // create the reply
    let reply = JoinVcReply {
        channel_id: channel.id.clone(),
        identity: client.identity.clone(),
        token: client.token.clone(),
        rtp: channel.router.rtp_capabilities().clone(),
    };

    // add the client to the global client list
    clients
        .lock()
        .unwrap()
        .insert(client.identity.clone(), client);

    Ok(Json(reply))
}
