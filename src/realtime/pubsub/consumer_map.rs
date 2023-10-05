use std::collections::{HashMap, HashSet};

use super::{consumer::EventConsumer, topic::Topic};

pub struct ConsumerMap {
    pub topic_to_cons: HashMap<Topic, HashSet<EventConsumer>>,
    // socket id to event consumer and subscribed topics
    pub socket_id_to_cons_and_topics: HashMap<String, (EventConsumer, Vec<Topic>)>,
}

impl ConsumerMap {
    pub fn new() -> Self {
        Self {
            topic_to_cons: HashMap::new(),
            socket_id_to_cons_and_topics: HashMap::new(),
        }
    }

    pub fn add_consumer(&mut self, consumer: EventConsumer) {
        self.add_consumer_with_topics(consumer, Vec::new());
    }

    pub fn add_consumer_with_topics(&mut self, consumer: EventConsumer, topics: Vec<Topic>) {
        self.socket_id_to_cons_and_topics
            .insert(consumer.socket.id.clone(), (consumer, topics));
    }

    pub fn remove_consumer(&mut self, socket_id: &str) {
        if let Some((consumer, topics)) = self.socket_id_to_cons_and_topics.remove(socket_id) {
            for topic in topics {
                if let Some(consumers) = self.topic_to_cons.get_mut(&topic) {
                    consumers.remove(&consumer);
                }
            }
        }
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
