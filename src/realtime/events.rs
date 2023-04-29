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
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};

use crate::{
    auth::user::{UserEx, User},
    realtime::{
        consumer::EventConsumer,
        consumer_manager::EventConsumerManager,
        topic::{Topic, TopicType},
    }, DB,
};

use super::socket::Socket;

#[get("/events/ws")]
pub async fn events_ws(
    user: UserEx,
    ecm: Data<EventConsumerManager>,
    db: DB,
    req: HttpRequest,
    body: Payload,
) -> Result<HttpResponse, Error> {
    let on_message_handler: Box<dyn Fn(String) + Send + Sync + 'static>;
    let on_close_handler;

    {
        let ecm = ecm.clone();
        let user = user.clone();
        let db = db.clone();

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
                        // try to subscribe
                        ecm.subscribe(&user.id, topic).unwrap_or(());
                    }
                }
            }

            if let Some(Some(array)) = msg.get("unsub").map(|v| v.as_array()) {
                array.iter().for_each(|v| {
                    if let Some(Ok(topic)) = v.as_str().map(|s| s.parse()) {
                        // try to unsubscribe
                        ecm.unsubscribe(&user.id, &topic).unwrap_or(());
                    }
                })
            }
        });
    }

    {
        let user_id = user.id.clone();
        let ecm = ecm.clone();
        on_close_handler = Box::new(move |_| {
            ecm.remove_consumer(&user_id);
        });
    }

    let (socket, response) = Socket::new_arc_from_request(
        &req,
        body,
        // on message
        Some(on_message_handler),
        // on close
        Some(on_close_handler),
    )?;

    ecm.add_consumer(EventConsumer::new(user.id.clone(), socket));

    Ok(response)
}
