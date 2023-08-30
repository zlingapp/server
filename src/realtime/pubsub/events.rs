/*

Message:

{
    "sub": [
        "channel:cfb8vg91ydoGe8GL4QJgz",
        "guild:8PrfzKpuUIYgc6dfhYSgP"
    ],
    "unsub": [
        "channel:cfb8vg91ydoGe8GL4QJgz",
        "guild:8PrfzKpuUIYgc6dfhYSgP"
    ],
    "message": {
        "content": "Hello, world!",
        "channel_id": "cfb8vg91ydoGe8GL4QJgz",
    }
}

 */

use std::sync::Arc;

use actix_web::{
    error::ErrorUnauthorized,
    get,
    web::{Data, Payload, Query},
    Error, HttpRequest, HttpResponse,
};
use serde::Deserialize;

use crate::{
    auth::{access_token::AccessToken, token::Token},
    realtime::{
        pubsub::{consumer::EventConsumer, consumer_manager::EventConsumerManager},
        socket::Socket,
    },
};

#[derive(Deserialize)]
pub struct TokenInQuery {
    auth: String,
}

/// Event socket
/// 
/// Used to subscribe to certain generators of events (called "topics") and
/// receive information within these topics.
/// 
/// For example, this is used to receive messages from others in real time.
/// 
/// ## Subscribing
/// Subscribe to the topic of type `channel` with id `j_NNyhSbOl1AwqCTMAZ2G`.
/// ```js
/// {
///     "sub": [ "channel:j_NNyhSbOl1AwqCTMAZ2G" ]
/// }
/// ```
/// 
/// Zling will now notify you about messages sent in channel
/// `j_NNyhSbOl1AwqCTMAZ2G` and updates to the channel itself.
/// 
/// Here is what an event of type `message` might look like.
/// ```js
/// {
///     "topic": {
///         "id": "j_NNyhSbOl1AwqCTMAZ2G",
///         "type": "channel"
///     }
///     "event": {
///         "type": "message"
///         "id": "uIqNlwPDYrz9iou_ycKvd",
///         "createdAt": "2023-08-29T17:29:22.343533Z",
///         "content": "test",
///         "author": {
///             "avatar": "/api/media/QoJXQnwJY1CfQj2L0H9gH/avatar.png",
///             "id": "kEBbg9_IZXajYRevn7cUS",
///             "username": "someone#1234"
///         },
///     },
/// }
/// ```
/// 
/// ### Unsubscribing
/// Unsubscribing to topics is quite similar. It can be done like so:
/// ```js
/// {
///     "unsub": [ "channel:j_NNyhSbOl1AwqCTMAZ2G" ]
/// }
/// ```
#[utoipa::path(
    tag = "pubsub",
    params(
        ("auth" = AccessToken, Query, description = "Access token")
    ),
)]
#[get("/events/ws")]
pub async fn events_ws(
    ecm: Data<EventConsumerManager>,
    req: HttpRequest,
    query: Query<TokenInQuery>,
    body: Payload,
) -> Result<HttpResponse, Error> {
    let token = match query.auth.parse::<Token>() {
        Ok(token) => AccessToken::from_existing(token),
        Err(_) => {
            return Err(ErrorUnauthorized("authentication_required"));
        }
    }
    .ok_or(ErrorUnauthorized("authentication_required"))?;

    let on_message_handler: Box<dyn Fn(String) + Send + Sync + 'static>;
    let on_close_handler;

    let token = Arc::new(token);

    let socket_id = nanoid::nanoid!();

    {
        let ecm = ecm.clone();
        let socket_id = socket_id.clone();

        on_message_handler = Box::new(move |msg: String| {
            // message can contain keys `sub`, `unsub`, and `message`
            // `sub` and `unsub` are arrays of topics to subscribe/unsubscribe to
            // `message` is a message to send

            let msg = match serde_json::from_str::<serde_json::Value>(&msg) {
                Ok(msg) => msg,
                Err(err) => {
                    log::error!("failed to parse message: {}", err);
                    return;
                }
            };

            let msg = match msg.as_object() {
                Some(msg) => msg,
                None => {
                    log::error!("message is not an object: {}", msg);
                    return;
                }
            };

            if let Some(Some(array)) = msg.get("sub").map(|v| v.as_array()) {
                for v in array {
                    if let Some(Ok(topic)) = v.as_str().map(|s| s.parse()) {
                        // todo: do permission check here

                        // susbcribe to topic
                        ecm.subscribe(&socket_id, topic).unwrap_or(());
                    }
                }
            }

            if let Some(Some(array)) = msg.get("unsub").map(|v| v.as_array()) {
                array.iter().for_each(|v| {
                    if let Some(Ok(topic)) = v.as_str().map(|s| s.parse()) {
                        // todo: do permission check here

                        // unsubscribe from topic
                        ecm.unsubscribe(&socket_id, &topic).unwrap_or(());
                    }
                })
            }
        });
    }

    {
        let socket_id = socket_id.clone();
        let ecm = ecm.clone();
        on_close_handler = Box::new(move |_| {
            ecm.remove_consumer(&socket_id);
        });
    }

    let (socket, response) = Socket::new_arc_from_request(
        socket_id,
        &req,
        body,
        // on message
        Some(on_message_handler),
        // on close
        Some(on_close_handler),
    )?;

    ecm.add_consumer(EventConsumer::new(token.user_id.clone(), socket));

    Ok(response)
}
