use std::sync::Arc;

use actix_web::{
    get,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use log::{error, info};
use mediasoup::rtp_parameters::MediaKind;
use serde::Deserialize;
use serde_json::json;

use crate::{
    auth::user::PublicUserInfo,
    realtime::socket::{SendFailureReason, Socket},
    voice::{
        channel::VoiceChannel,
        client::{VoiceClient, VoiceClientEx},
        VoiceChannels, VoiceClients,
    },
};

#[derive(Deserialize)]
// note: cannot use IntoParams here as they are doomed to be path parameters
// instead of query for some reason, "why" is beyond me
pub struct IdAndToken {
    /// RTC Identity
    pub i: String,
    /// RTC Token
    pub t: String,
}

/// Voice WebSocket
///
/// Connect to this to receive real-time events about peers in the current voice
/// chat. Connection is mandatory and must be made within 10 seconds of a
/// `/voice/join` request.
///
/// As `WebSocket`s in the browser are unable to supply additional headers, the
/// RTC Identity and Token must be sent in the query parameters of the intitial
/// request, in the form `?i=identity` and `?t=token`.
///
/// ## Messages
/// The connection is read-only, client to server signalling does not happen
/// over this socket at all. These are the messages your client may receive and
/// should respond to.
///
/// #### Peer Joined
/// ```js
/// {
///     "type": "client_connected",
///     "identity": "YDjdIc06vuaVQZy4LS7hb",
///     "user": { ... }, // contains id, name, avatar, etc.
/// }
/// ```
///
/// #### Peer Disconnected
/// ```js
/// {
///     "type": "client_disconnected",
///     "identity": "YDjdIc06vuaVQZy4LS7hb",
/// }
/// ```
///
/// #### New Producer
/// A peer created a new producer, you may consume it.
/// ```js
/// {
///     "type": "new_producer",
///     "identity": "YDjdIc06vuaVQZy4LS7hb",
///     "producer_id": "...",
///     "producer_kind": "audio|video",
/// }
/// ```
#[utoipa::path(
    tag = "voice",
    security(("voice" = [])),
    params(
        ("i" = String, Query, description = "RTC Identity"),
        ("t" = String, Query, description = "RTC Token"),
    ),
    responses(
        (status = 101, description = "WebSocket connected successfully"),
        (status = BAD_REQUEST, description = "Already connected"),
    )
)]
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
            "user": PublicUserInfo::from(client.user.clone()),
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
