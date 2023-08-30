use std::{hash::Hash, sync::Arc};

use crate::realtime::socket::Socket;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct EventConsumer {
    pub user_id: String,
    pub socket: Arc<Socket>,
}

impl EventConsumer {
    pub fn new(user_id: String, socket: Arc<Socket>) -> Self {
        Self { user_id, socket }
    }
}
