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

use actix_web::{
    get,
    web::{Data, Payload, Query},
    HttpRequest, HttpResponse,
};
use serde::Deserialize;

use crate::{
    auth::{access_token::AccessToken, token::Token},
    error::IntoHandlerErrorResult,
    realtime::{pubsub::pubsub::PubSub, socket::Socket},
};

use super::topic::Topic;

#[derive(Deserialize)]
pub struct TokenInQuery {
    auth: String,
}

/// Sent by the client on the event socket.
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum EventSocketRequest {
    #[serde(rename = "sub")]
    Subscribe { topics: Vec<Topic> },
    #[serde(rename = "unsub")]
    Unsubscribe { topics: Vec<Topic> },
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
///     "type": "sub",
///     "topics": [{ "id": "j_NNyhSbOl1AwqCTMAZ2G", "type": "channel" } }]
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
///             "avatar": "/media/QoJXQnwJY1CfQj2L0H9gH/avatar.png",
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
///     "type": "unsub",
///     "topics": [{ "id": "j_NNyhSbOl1AwqCTMAZ2G", "type": "channel" } }]
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
    pubsub: Data<PubSub>,
    req: HttpRequest,
    query: Query<TokenInQuery>,
    body: Payload,
) -> Result<HttpResponse, actix_web::Error> {
    // get token from query
    let token = query.auth.parse::<Token>().or_err(401)?;
    let token = AccessToken::from_existing(token).or_err(401)?;
    let id = token.user_id.clone(); // Save for registering consumer

    // set up handlers
    let on_message_handler: Box<dyn Fn(String) + Send + Sync + 'static>;
    let on_close_handler;

    // generate random socket id
    let socket_id = nanoid::nanoid!();

    {
        let socket_id = socket_id.clone();
        let pubsub = pubsub.clone();
        on_message_handler = Box::new(move |msg: String| {
            let socket_id = socket_id.clone();
            let pubsub = pubsub.clone();
            actix_rt::spawn(async move {
                if let Ok(esr) = serde_json::from_str::<EventSocketRequest>(&msg) {
                    use EventSocketRequest::*;
                    match esr {
                        Subscribe { topics } => {
                            for topic in topics {
                                pubsub.subscribe(&socket_id, topic).await.unwrap_or(());
                            }
                        }
                        Unsubscribe { topics } => {
                            for topic in topics {
                                pubsub.unsubscribe(&socket_id, topic).await.unwrap_or(());
                            }
                        }
                    }
                }
            });
        });
    }
    {
        let pubsub = pubsub.clone();
        let socket_id = socket_id.clone();
        let uid = token.user_id.clone();
        on_close_handler = Box::new(move |_| {
            let pubsub = pubsub.clone();
            let socket_id = socket_id.clone();
            let uid = uid.clone();
            actix_rt::spawn(async move {
                pubsub.remove_socket(&uid, &socket_id).await;
            });
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

    pubsub.add_socket(id, socket).await;

    Ok(response)
}
