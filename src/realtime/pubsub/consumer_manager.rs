use futures::future::join_all;
use serde::Serialize;
use serde_json::json;

use crate::{
    auth::user::{PublicUserInfo, User},
    messaging::message::Message,
};

use super::{
    consumer::EventConsumer,
    consumer_map::ConsumerMap,
    topic::{Topic, TopicType},
};
use std::sync::RwLock;

pub struct EventConsumerManager {
    consumers: RwLock<ConsumerMap>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event<'l> {
    /// Something changed in the list of channels in a guild.
    ChannelListUpdate,
    /// A new message was sent.
    Message(&'l Message),
    /// A message was deleted.
    DeleteMessage { id: &'l str },
    /// A user started typing in a channel.
    Typing { user: &'l PublicUserInfo },
}

impl EventConsumerManager {
    pub fn new() -> Self {
        Self {
            consumers: RwLock::new(ConsumerMap::new()),
        }
    }

    pub async fn broadcast(&self, topic: &Topic, event: Event<'_>) {
        let consumers = self.consumers.read().unwrap();
        if let Some(consumers) = consumers.topic_to_cons.get(topic) {
            let mut futures = Vec::with_capacity(consumers.len());

            for consumer in consumers {
                futures.push(
                    consumer.socket.send(
                        json!({
                            "topic": topic,
                            "event": event,
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

    pub fn remove_consumer(&self, socket_id: &str) {
        self.consumers.write().unwrap().remove_consumer(socket_id);
    }

    pub fn subscribe(&self, socket_id: &str, topic: Topic) -> Result<(), ()> {
        self.consumers.write().unwrap().subscribe(socket_id, topic)
    }

    pub fn unsubscribe(&self, socket_id: &str, topic: &Topic) -> Result<(), ()> {
        self.consumers
            .write()
            .unwrap()
            .unsubscribe(socket_id, topic)
    }

    pub async fn notify_of_new_message(&self, channel_id: &str, message: &Message) {
        self.broadcast(
            &Topic::new(TopicType::Channel, channel_id.to_owned()),
            Event::Message(message),
        )
        .await;
    }

    pub async fn notify_guild_channel_list_update(&self, guild_id: &str) {
        self.broadcast(
            &Topic::new(TopicType::Guild, guild_id.to_string()),
            Event::ChannelListUpdate,
        )
        .await;
    }

    pub async fn send_typing(&self, channel_id: &str, user: &User) {
        let topic = Topic::new(TopicType::Channel, channel_id.to_string());

        self.broadcast(
            &topic,
            Event::Typing {
                user: &PublicUserInfo::from(user.clone()),
            },
        )
        .await;
    }

    pub async fn notify_message_deleted(&self, channel_id: &str, message_id: &str) {
        let topic = Topic::new(TopicType::Channel, channel_id.to_string());

        self.broadcast(&topic, Event::DeleteMessage { id: message_id })
            .await;
    }
}
