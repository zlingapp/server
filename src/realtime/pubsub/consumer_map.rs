use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::topic::Topic;
use crate::realtime::socket::Socket;

pub struct ConsumerMap {
    pub topic_to_cons: HashMap<Topic, HashSet<Arc<Socket>>>,
    // socket id to event consumer and subscribed topics
    pub socket_id_to_cons_and_topics: HashMap<String, (Arc<Socket>, Vec<Topic>)>,
    // user id to sockets
    pub user_id_to_sockets: HashMap<String, Vec<Arc<Socket>>>,
}

impl ConsumerMap {
    pub fn new() -> Self {
        Self {
            topic_to_cons: HashMap::new(),
            socket_id_to_cons_and_topics: HashMap::new(),
            user_id_to_sockets: HashMap::new(),
        }
    }

    pub fn add_consumer(&mut self, user_id: String, consumer: Arc<Socket>) {
        self.user_id_to_sockets
            .entry(user_id)
            .or_insert(Vec::new())
            .push(consumer.clone());
        self.socket_id_to_cons_and_topics
            .insert(consumer.id.clone(), (consumer, Vec::new()));
        println!(
            "{:?}",
            self.user_id_to_sockets
                .iter()
                .map(|e| (
                    e.0.clone(),
                    e.1.iter().map(|i| i.id.clone()).collect::<String>()
                ))
                .collect::<HashMap<String, String>>()
        );
    }

    pub fn remove_consumer(&mut self, user_id: &str, socket_id: &str) {
        if let Some((consumer, topics)) = self.socket_id_to_cons_and_topics.remove(socket_id) {
            for topic in topics {
                if let Some(consumers) = self.topic_to_cons.get_mut(&topic) {
                    consumers.remove(&consumer);
                }
            }
        }
        self.user_id_to_sockets
            .entry(user_id.into())
            .and_modify(|e| {
                e.iter()
                    .position(|i| i.id == socket_id)
                    .map(|i| e.remove(i));
            });
        println!(
            "{:?}",
            self.user_id_to_sockets
                .iter()
                .map(|e| (
                    e.0.clone(),
                    e.1.iter().map(|i| i.id.clone()).collect::<String>()
                ))
                .collect::<HashMap<String, String>>()
        );
    }

    pub fn subscribe(&mut self, socket_id: &str, topic: Topic) -> Result<(), ()> {
        if let Some((ref consumer, topics)) = self.socket_id_to_cons_and_topics.get_mut(socket_id) {
            topics.push(topic.clone());

            if let Some(consumers) = self.topic_to_cons.get_mut(&topic) {
                consumers.insert(consumer.clone());
            } else {
                self.topic_to_cons
                    .insert(topic, HashSet::from([consumer.clone()]));
            }
        } else {
            return Err(());
        };
        Ok(())
    }

    pub fn unsubscribe(&mut self, socket_id: &str, topic: &Topic) -> Result<(), ()> {
        if let Some((consumer, topics)) = self.socket_id_to_cons_and_topics.get_mut(socket_id) {
            topics.retain(|t| t != topic);

            if let Some(consumers) = self.topic_to_cons.get_mut(&topic) {
                consumers.remove(consumer);
            }
        } else {
            return Err(());
        }
        Ok(())
    }
}
