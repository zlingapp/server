use std::{sync::{RwLock, Arc}, collections::HashMap, ops::Deref};

use actix_web::FromRequest;
use log::warn;
use nanoid::nanoid;
use sqlx::query;

use crate::{db::DB, crypto};

use super::user::{SessionToken, User};

pub struct SessionManager {
    db: DB,
    // todo: use a cache with expiry and access ttl here
    sessions: RwLock<HashMap<SessionToken, Arc<User>>>
}

pub enum SessionAuthResult {
    Failure,
    Success {
        user: Arc<User>,
        session: SessionToken,
    },
}

impl SessionManager {
    pub fn new(db: DB) -> Self {
        Self {
            db,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn auth_new_session(&self, email: &str, password: &str) -> SessionAuthResult {
        let user = query!(
            "SELECT id, name, email, avatar, password FROM users WHERE email = $1",
            email
        )
        .fetch_one(&self.db.pool)
        .await;

        match user {
            Ok(record) => {
                if !crypto::verify(password, &record.password) {
                    return SessionAuthResult::Failure;
                }

                let user = Arc::new(User {
                    id: record.id,
                    name: record.name,
                    email: record.email,
                    avatar: record.avatar,
                });

                let session = nanoid!(64);

                self.sessions
                    .write()
                    .unwrap()
                    .insert(session.clone(), user.clone());

                return SessionAuthResult::Success { user, session };
            }
            Err(e) => {
                warn!("Failed to get user from db: {}", e);
                return SessionAuthResult::Failure;
            }
        }
    }

    pub fn get_user_by_session(&self, session: &str) -> Option<Arc<User>> {
        self.sessions.read().unwrap().get(session).cloned()
    }

    pub fn erase_session(&self, user: &User, session: &str) -> bool {
        {
            let lock = self.sessions.read().unwrap();
            let stored_user = lock.get(session).unwrap();
            if stored_user.id != user.id {
                return false;
            }
        }
        self.sessions.write().unwrap().remove(session).is_some()
    }
}

pub struct SessionEx(SessionToken);

impl Deref for SessionEx {
    type Target = SessionToken;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for SessionEx {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        use actix_web::error::*;
        use std::future::ready;

        let cookie = match req.cookie("Session") {
            Some(cookie) => cookie,
            None => return ready(Err(ErrorUnauthorized("access_denied"))),
        };

        ready(Ok(SessionEx(cookie.value().to_owned())))
    }
}