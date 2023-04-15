use std::{sync::Arc, time::Duration};

use actix_rt::time::sleep;
use actix_web::{
    error, get,
    web::{Data, Json, Query},
    Error, HttpResponse,
};
use derive_more::{Display, Error};

use log::warn;
use mediasoup::{rtp_parameters::RtpCapabilitiesFinalized, worker_manager::WorkerManager};
use serde::{Deserialize, Serialize};

use crate::{
    channel::create_channel,
    client::{Client, ClientEx},
    Channels, Clients,
};

#[derive(Deserialize)]
pub struct JoinVcQuery {
    // channel id
    c: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
pub async fn join_vc(
    clients: Data<Clients>,
    channels: Data<Channels>,
    wm: Data<WorkerManager>,
    query: Query<JoinVcQuery>,
) -> JoinVcResponse {
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

    // the socket watchdog checks if the client has connected to the websocket in time
    // if it hasn't, the client is removed from the channel silently
    {
        // two clones of the Arc<Client> are created here
        let client_inner = client.clone();
        let client_outer = client.clone();

        let clients_inner = clients.clone();

        // client_inner is moved into the async block
        // clients_inner is moved into the async block
        let handle = Some(actix_rt::spawn(async move {
            sleep(Duration::from_secs(10)).await;

            // if the client hasn't connected to the websocket yet, remove it from the channel
            if client_inner.socket_session.read().unwrap().is_none() {
                warn!("Client {} didn't connect to the websocket in time, removing it from the channel", client_inner.identity);
                client_inner
                    .channel
                    .erase_client(
                        &client_inner.identity,
                        clients_inner.into_inner(),
                        channels.into_inner(),
                    )
                    .await;
            }
        }));

        *client_outer.socket_watchdog_handle.lock().unwrap() = handle;
    }

    // add the client to the global client list
    clients
        .lock()
        .unwrap()
        .insert(client.identity.clone(), client);

    Ok(Json(reply))
}

#[get("/leave")]
pub async fn leave_vc(
    client: ClientEx,
    clients: Data<Clients>,
    channels: Data<Channels>,
) -> Result<HttpResponse, Error> {
    client
        .channel
        .disconnect_client(&client, clients.into_inner(), channels.into_inner())
        .await;
    Ok(HttpResponse::Ok().finish())
}
