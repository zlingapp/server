use std::sync::Arc;

use actix_web::{
    get,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use log::{error, info};
use mediasoup::rtp_parameters::MediaKind;
use serde_json::json;

use crate::{
    realtime::socket::{SendFailureReason, Socket},
    voice::{
        channel::VoiceChannel,
        client::{VoiceClient, VoiceClientEx},
        VoiceChannels, VoiceClients,
    },
};

#[get("/voice/ws")] // WARNING: before changing this path, make sure to change it in the client extractor!!!
pub async fn voice_events_ws(
    client: VoiceClientEx,
    clients: Data<VoiceClients>,
    channels: Data<VoiceChannels>,
    req: HttpRequest,
    body: Payload,
) -> Result<HttpResponse, Error> {
    if client.socket.read().unwrap().is_some() {
        return Err(actix_web::error::ErrorBadRequest("already_connected"));
    }

    // this is ugly but needed so the `move` callbacks below can access the client

    let on_message_handler;
    let on_close_handler;

    {
        // this is needed so the `move` callback below can access the client
        let weak_client = Arc::downgrade(&client);

        on_message_handler = Box::new(move |msg| {
            let client = match weak_client.upgrade() {
                Some(client) => client,
                None => {
                    // client is already deallocated
                    return;
                }
            };

            client.on_socket_message(msg)
        });
    }

    {
        // this is needed so the `move` callback below can access the client
        let weak_client = Arc::downgrade(&client);

        on_close_handler = Box::new(move |reason| {
            let client = match weak_client.upgrade() {
                Some(client) => client,
                None => {
                    // client is already deallocated
                    return;
                }
            };

            info!(
                "client[{:?}]: disconnected from event socket: {:?}",
                client.identity, reason
            );

            {
                // these arcs go in the closure below
                let client = client.clone();
                let clients = clients.clone();
                let channels = channels.clone();
                actix_rt::spawn(async move {
                    if clients.lock().unwrap().contains_key(&client.identity) {
                        // notify channel
                        info!("calling channel.disconnect_client");
                        client
                            .channel
                            .disconnect_client(&client, &clients, &channels)
                            .await;
                    }
                });
            }
        })
    }

    let (socket, response) = Socket::new_arc_from_request(
        nanoid::nanoid!(),
        &req,
        body,
        // on message
        Some(on_message_handler),
        // on close
        Some(on_close_handler),
    )?;

    info!(
        "client[{:?}]: connected to event socket: {:?}",
        client.identity, client.channel.id
    );

    *client.socket.write().unwrap() = Some(socket);
    client.channel.notify_client_joined(&client).await;

    Ok(response)
}

impl VoiceClient {
    pub fn on_socket_message(&self, msg: String) {
        info!(
            "client[{:?}]: received message on event socket: {:?}",
            self.identity, msg
        );
    }

    pub async fn send(&self, msg: String) -> Result<(), SendFailureReason> {
        match self.socket.read().unwrap().as_ref() {
            Some(socket) => socket.send(msg).await,
            None => Err(SendFailureReason::NoSession),
        }
    }
}

impl VoiceChannel {
    pub async fn send_to_all_except(&self, except: &VoiceClient, msg: String) {
        let clients = self.clients.lock().unwrap();
        for client in clients.iter() {
            if client.identity == except.identity {
                continue;
            }

            if client.send(msg.clone()).await.is_err() {
                // this shouldn't happen
                error!(
                    "tried to send to disconnected client: {:?}",
                    client.identity
                );
            };
        }
    }

    pub async fn notify_client_joined(&self, client: &VoiceClient) {
        // serialize the message
        let event = json!({
            "type": "client_connected",
            "identity": client.identity,
        });

        self.send_to_all_except(&client, event.to_string()).await;
    }

    pub async fn notify_client_left(&self, client: &VoiceClient) {
        // serialize the message
        let event = json!({
            "type": "client_disconnected",
            "identity": client.identity,
        });

        self.send_to_all_except(&client, event.to_string()).await;
    }

    pub async fn notify_new_producer(
        &self,
        client: &VoiceClient,
        producer_id: String,
        producer_kind: MediaKind,
    ) {
        // serialize the message
        let event = json!({
            "type": "new_producer",
            "identity": client.identity,
            "producer_id": producer_id,
            "producer_kind": producer_kind,
        });

        self.send_to_all_except(&client, event.to_string()).await;
    }
}
