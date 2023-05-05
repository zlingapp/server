use futures::future::join_all;
use serde_json::{json, Value};
use sqlx::types::chrono::NaiveDateTime;

use crate::auth::user::User;

use super::{
    consumer::EventConsumer,
    consumer_map::ConsumerMap,
    topic::{Topic, TopicType},
};
use std::sync::RwLock;

pub struct EventConsumerManager {
    consumers: RwLock<ConsumerMap>,
}

impl EventConsumerManager {
    pub fn new() -> Self {
        Self {
            consumers: RwLock::new(ConsumerMap::new()),
        }
    }

    pub async fn broadcast(&self, topic: &Topic, data: Value) {
        let consumers = self.consumers.read().unwrap();
        if let Some(consumers) = consumers.topic_to_cons.get(topic) {
            let mut futures = Vec::with_capacity(consumers.len());

            for consumer in consumers {
                futures.push(
                    consumer.socket.send(
                        json!({
                            "topic": topic,
                            "event": data,
                        })
                        .to_string(),
                    ),
                );
            }

            join_all(futures).await;
        }
    }
}

// re-export ConsumerMap methods
impl EventConsumerManager {
    pub fn add_consumer(&self, consumer: EventConsumer) {
        self.consumers.write().unwrap().add_consumer(consumer);
    }

    pub fn remove_consumer(&self, user_id: &str) {
        self.consumers.write().unwrap().remove_consumer(user_id);
    }

    pub fn subscribe(&self, user_id: &str, topic: Topic) -> Result<(), ()> {
        self.consumers.write().unwrap().subscribe(user_id, topic)
    }

    pub fn unsubscribe(&self, user_id: &str, topic: &Topic) -> Result<(), ()> {
        self.consumers.write().unwrap().unsubscribe(user_id, topic)
    }

    pub async fn notify_of_new_message(
        &self,
        user: &User,
        channel_id: &str,
        message_id: &str,
        content: &str,
        created_at: &NaiveDateTime,
        author_nickname: Option<String>,
    ) {
        let payload = serde_json::json!({
            "type": "message",
            "id": message_id,
            "author": {
                "id": user.id,
                "username": user.name,
                "avatar": user.avatar,
                "nickname": author_nickname,
            },
            "created_at": created_at.to_string(),
            "content": content,
        });

        self.broadcast(
            &Topic::new(TopicType::Channel, channel_id.to_owned()),
            payload,
        )
        .await;
    }
}
