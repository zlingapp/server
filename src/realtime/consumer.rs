use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::auth::user::UserId;

use super::socket::Socket;

#[derive(Clone)]
pub struct EventConsumer {
    pub user_id: UserId,
    pub socket: Arc<Socket>,
}

impl Hash for EventConsumer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.user_id.hash(state);
    }
}

impl PartialEq for EventConsumer {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl Eq for EventConsumer {}

impl EventConsumer {
    pub fn new(user_id: UserId, socket: Arc<Socket>) -> Self {
        Self { user_id, socket }
    }
}