use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::topic::Topic;
use crate::realtime::socket::Socket;

pub struct PubSubMap {
    // topic -> sockets subscribed to it
    pub topic_to_sockets: HashMap<Topic, HashSet<Arc<Socket>>>,
    // socket id -> the actual socket and topics it's subscribed to
    pub socket_id_to_socket_and_topics: HashMap<String, (Arc<Socket>, Vec<Topic>)>,
    // user id -> sockets that user has open (probably multiple clients)
    pub user_id_to_sockets: HashMap<String, Vec<Arc<Socket>>>,
}

impl PubSubMap {
    pub fn new() -> Self {
        Self {
            topic_to_sockets: HashMap::new(),
            socket_id_to_socket_and_topics: HashMap::new(),
            user_id_to_sockets: HashMap::new(),
        }
    }

    pub fn add_socket(&mut self, user_id: String, socket: Arc<Socket>) {
        self.user_id_to_sockets
            .entry(user_id)
            .or_insert(Vec::new())
            .push(socket.clone());

        self.socket_id_to_socket_and_topics
            .insert(socket.id.clone(), (socket, Vec::new()));
    }

    pub fn remove_socket(&mut self, user_id: &str, socket_id: &str) {
        if let Some((socket, topics)) = self.socket_id_to_socket_and_topics.remove(socket_id) {
            for topic in topics {
                self.topic_to_sockets.entry(topic).and_modify(|sockets| {
                    sockets.remove(&socket);
                });
            }
        }

        self.user_id_to_sockets
            .entry(user_id.into())
            .and_modify(|e| {
                e.retain(|i| i.id != socket_id);
            });
    }

    pub fn subscribe(&mut self, socket_id: &str, topic: Topic) -> Result<(), ()> {
        if let Some((ref socket, topics)) = self.socket_id_to_socket_and_topics.get_mut(socket_id) {
            topics.push(topic.clone());

            if let Some(sockets) = self.topic_to_sockets.get_mut(&topic) {
                sockets.insert(socket.clone());
            } else {
                self.topic_to_sockets
                    .insert(topic, HashSet::from([socket.clone()]));
            }
        } else {
            return Err(());
        };
        Ok(())
    }

    pub fn unsubscribe(&mut self, socket_id: &str, topic: &Topic) -> Result<(), ()> {
        if let Some((socket, topics)) = self.socket_id_to_socket_and_topics.get_mut(socket_id) {
            topics.retain(|t| t != topic);

            if let Some(sockets) = self.topic_to_sockets.get_mut(&topic) {
                sockets.remove(socket);
            }
        } else {
            return Err(());
        }
        Ok(())
    }
}
