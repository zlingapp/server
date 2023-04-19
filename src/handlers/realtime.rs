use actix_web::{
    get,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_ws::{Closed, Message};
use futures::StreamExt;
use log::{error, info};
use mediasoup::rtp_parameters::MediaKind;
use serde_json::json;

use crate::{
    channel::Channel,
    client::{Client, ClientEx},
    Channels, Clients,
};

#[get("/ws")] // WARNING: before changing this path, make sure to change it in the client extractor!!!
pub async fn events_ws(
    client: ClientEx,
    clients: Data<Clients>,
    channels: Data<Channels>,
    req: HttpRequest,
    body: Payload,
) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    *client.socket_session.write().unwrap() = Some(session.clone());

    actix_rt::spawn(async move {
        // notify channel
        client.channel.notify_client_joined(&client).await;

        info!(
            "client[{:?}]: connected to event socket: {:?}",
            client.identity, client.channel.id
        );

        // notify client of existing producers
        // for other_client in client.channel.clients.lock().unwrap().iter() {
        //     for producer in other_client.producers.lock().unwrap().values() {
        //         let msg = json!({
        //             "type": "new_producer",
        //             "identity": other_client.identity,
        //             "producer_id": producer.id(),
        //             "producer_kind": producer.kind(),
        //         });

        //         let client_ = client.clone();
        //         actix_rt::spawn(async move {
        //             client_.send(msg.to_string()).await.unwrap();
        //         });
        //     }
        // }

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Message::Text(s) => client.on_message_from_client(s.to_string()).await,
                _ => break,
            }
        }

        // disconnect client here
        let _ = session.close(None).await;
        *client.socket_session.write().unwrap() = None;

        /* this check is necessary because the client might have already been removed
         for example, if the client intentionally disconnected:
         1. client GET's /leave, leading to a call to Channel::disconnect_client
         2. Channel::disconnect_client
            - removes the client from its list of clients
            - removes the client from the list of clients in the global `Clients` map
            - notifies the other clients that the client disconnected
        3. the client closes its websocket connection
        4. this code below runs

        if the client was removed from the global `Clients` map in step 2, then
        the check below will fail and Channel::disconnect_client will not be called again
        */
        if clients.lock().unwrap().contains_key(&client.identity) {
            // notify channel
            client
                .channel
                .disconnect_client(&client, clients.into_inner(), channels.into_inner())
                .await;
        }
    });

    Ok(response)
}

impl Client {
    pub async fn on_message_from_client(&self, msg: String) {
        if msg == "heartbeat" {
            *self.last_ping.write().unwrap() = Some(std::time::Instant::now());
            return;
        }

        info!(
            "client[{:?}]: received message on event socket: {:?}",
            self.identity, msg
        );
    }

    pub async fn send(&self, msg: String) -> Result<(), Closed> {
        if let Some(session) = self.socket_session.write().unwrap().as_mut() {
            session.text(msg).await?;
        }

        Ok(())
    }
}

impl Channel {
    pub async fn send_to_all_except(&self, except: &Client, msg: String) {
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

    pub async fn notify_client_joined(&self, client: &Client) {
        // serialize the message
        let event = json!({
            "type": "client_connected",
            "identity": client.identity,
        });

        self.send_to_all_except(&client, event.to_string()).await;
    }

    pub async fn notify_client_left(&self, client: &Client) {
        // serialize the message
        let event = json!({
            "type": "client_disconnected",
            "identity": client.identity,
        });

        self.send_to_all_except(&client, event.to_string()).await;
    }

    pub async fn notify_new_producer(
        &self,
        client: &Client,
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

    pub async fn notify_producer_closed(&self, client: &Client, producer_id: String) {
        // serialize the message
        let event = json!({
            "type": "producer_closed",
            "identity": client.identity,
            "producer_id": producer_id,
        });

        self.send_to_all_except(&client, event.to_string()).await;
    }
}
