use futures::future::join_all;
use serde::Serialize;
use serde_json::json;

use crate::{
    auth::user::{PublicUserInfo, User},
    messaging::message::Message,
    realtime::socket::Socket,
};

use super::{
    pubsub_map::PubSubMap,
    topic::{Topic, TopicType},
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PubSub {
    map: RwLock<PubSubMap>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event<'l> {
    /// Something changed in the list of channels in a guild.
    ChannelListUpdate,
    /// Something changed in the list of members in a guild.
    MemberListUpdate,
    /// A new message was sent.
    Message(&'l Message),
    /// A message was deleted.
    DeleteMessage { id: &'l str },
    /// A user started typing in a channel.
    Typing { user: &'l PublicUserInfo },

    /// Some sort of update to a friend request. Clients should keep track of these
    /// Options
    /// - Someone sent you a friend request
    /// - Someone accepted your friend request
    FriendRequestUpdate { user: &'l PublicUserInfo },
    /// A friend request has been deleted. Clients should keep track of theses
    /// Options
    /// - Someone denied your friend request
    /// - Someone cancelled a friend request they sent to your
    FriendRequestRemove { user: &'l PublicUserInfo },
    /// Someone severed all ties with you
    FriendRemove { user: &'l PublicUserInfo },
}

impl PubSub {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(PubSubMap::new()),
        }
    }

    pub async fn broadcast(&self, topic: &Topic, event: Event<'_>) {
        let map = self.map.read().await;

        if let Some(subscribed_sockets) = map.topic_to_sockets.get(topic) {
            let mut futures = Vec::with_capacity(subscribed_sockets.len());

            for socket in subscribed_sockets {
                futures.push(
                    socket.send(
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
    pub async fn send_to(&self, user_id: &str, event: Event<'_>) {
        let map = self.map.read().await;

        if let Some(user_sockets) = map.user_id_to_sockets.get(user_id) {
            let mut futures = Vec::with_capacity(user_sockets.len());

            for socket in user_sockets {
                futures.push(
                    socket.send(
                        json!({
                            "topic": Topic::new(TopicType::User, user_id.into()),
                            "event": event
                        })
                        .to_string(),
                    ),
                )
            }

            join_all(futures).await;
        }
    }

    // re-export PubSubMap methods
    pub async fn add_socket(&self, user_id: String, socket: Arc<Socket>) {
        self.map.write().await.add_socket(user_id, socket);
    }

    pub async fn remove_socket(&self, user_id: &str, socket_id: &str) {
        self.map.write().await.remove_socket(user_id, socket_id);
    }

    pub async fn subscribe(&self, socket_id: &str, topic: Topic) -> Result<(), ()> {
        self.map.write().await.subscribe(socket_id, topic)
    }

    pub async fn unsubscribe(&self, socket_id: &str, topic: Topic) -> Result<(), ()> {
        self.map.write().await.unsubscribe(socket_id, &topic)
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

    pub async fn notify_guild_member_list_update(&self, guild_id: &str) {
        self.broadcast(
            &Topic::new(TopicType::Guild, guild_id.to_string()),
            Event::MemberListUpdate,
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
