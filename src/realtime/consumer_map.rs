use std::collections::{HashMap, HashSet};

use crate::auth::user::UserId;

use super::{consumer::EventConsumer, topic::Topic};

pub struct ConsumerMap {
    pub topic_to_cons: HashMap<Topic, HashSet<EventConsumer>>,
    pub id_to_cons_and_topics: HashMap<UserId, (EventConsumer, Vec<Topic>)>,
}

impl ConsumerMap {
    pub fn new() -> Self {
        Self {
            topic_to_cons: HashMap::new(),
            id_to_cons_and_topics: HashMap::new(),
        }
    }

    pub fn add_consumer(&mut self, consumer: EventConsumer) {
        self.add_consumer_with_topics(consumer, Vec::new());
    }

    pub fn add_consumer_with_topics(&mut self, consumer: EventConsumer, topics: Vec<Topic>) {
        self.id_to_cons_and_topics
            .insert(consumer.user_id.clone(), (consumer, topics));
    }

    pub fn remove_consumer(&mut self, user_id: &str) {
        if let Some((consumer, topics)) = self.id_to_cons_and_topics.remove(user_id) {
            for topic in topics {
                if let Some(consumers) = self.topic_to_cons.get_mut(&topic) {
                    consumers.remove(&consumer);
                }
            }
        }
    }

    pub fn subscribe(&mut self, user_id: &str, topic: Topic) -> Result<(), ()> {
        if let Some((ref consumer, topics)) = self.id_to_cons_and_topics.get_mut(user_id) {
            topics.push(topic.clone());

            if let Some(consumers) = self.topic_to_cons.get_mut(&topic) {
                consumers.insert(consumer.clone());
            } else {
                self.topic_to_cons.insert(topic, HashSet::from([consumer.clone()]));
            }
        } else {
            return Err(());
        };
        Ok(())
    }

    pub fn unsubscribe(&mut self, user_id: &str, topic: &Topic) -> Result<(), ()> {
        if let Some((consumer, topics)) = self.id_to_cons_and_topics.get_mut(user_id) {
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
