use std::{hash::Hash, sync::Arc};

use crate::{auth::user::UserId, realtime::socket::Socket};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct EventConsumer {
    pub user_id: UserId,
    pub socket: Arc<Socket>,
}

impl EventConsumer {
    pub fn new(user_id: UserId, socket: Arc<Socket>) -> Self {
        Self { user_id, socket }
    }
}
