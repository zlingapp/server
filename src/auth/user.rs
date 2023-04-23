use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, RwLock},
};

use std::ops::Deref;

use actix_web::{web::Data, FromRequest};
use futures::Future;
use log::warn;
use nanoid::nanoid;
use rand::Rng;
use sqlx::{postgres::PgQueryResult, query};

use crate::{auth::crypto, DB};

pub type UserId = String;
pub type SessionToken = String;

pub struct User {
    pub id: UserId,
    pub name: String,
    pub avatar: String,
    pub email: String,
}

impl User {
    pub async fn register_in_db(&self, db: &DB, password: &str) -> Result<bool, sqlx::Error> {
        let rows_affected = query!(
            r#"
                INSERT INTO users (id, name, email, avatar, password) 
                SELECT $1, $2, $3, $4, $5 
                FROM (SELECT 1) AS t
                WHERE NOT EXISTS (SELECT 1 FROM users WHERE email = $3)
            "#,
            self.id,
            self.name,
            self.email,
            self.avatar,
            crypto::hash(password)
        )
        .execute(db.as_ref())
        .await?.rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn fetch_by_id(id: &str, db: &DB) -> Option<Self> {
        let user = query!(
            "SELECT id, name, email, avatar FROM users WHERE id = $1",
            id
        )
        .fetch_one(db.as_ref())
        .await;

        match user {
            Ok(user) => Some(Self {
                id: user.id,
                name: user.name,
                email: user.email,
                avatar: user.avatar,
            }),
            Err(e) => {
                warn!("Failed to get user from db: {}", e);
                None
            }
        }
    }
}

pub struct UserManager {
    db: DB,
    // todo: use a cache with expiry and access ttl here
    sessions: RwLock<HashMap<SessionToken, Arc<User>>>,
}

// need to find a way to generate a unique discriminator...
fn generate_discrim() -> String {
    let mut rng = rand::thread_rng();

    String::from_iter([
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
    ])
}

pub enum AuthResult {
    Failure,
    Success {
        user: Arc<User>,
        session: SessionToken,
    },
}

impl UserManager {
    pub fn new(db: DB) -> Self {
        Self {
            db,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    // todo: implement db stuff
    pub async fn register_user(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Option<Arc<User>> {
        // todo: validate some regex here...
        let user = Arc::new(User {
            id: nanoid!(),
            name: String::from_iter([name, "#", &generate_discrim()]),
            email: email.to_owned(),
            avatar: "https://placehold.co/32".to_owned(),
        });

        match user.register_in_db(&self.db, password).await {
            Ok(did_create) => {
                if !did_create {
                    // user already exists
                    return None;
                }
                Some(user)
            },
            Err(e) => {
                warn!("Failed to create user: {}", e);
                None
            }
        }
    }

    pub async fn auth_new_session(&self, email: &str, password: &str) -> AuthResult {
        let user = query!(
            "SELECT id, name, email, avatar, password FROM users WHERE email = $1",
            email
        )
        .fetch_one(self.db.as_ref())
        .await;

        match user {
            Ok(record) => {
                if !crypto::verify(password, &record.password) {
                    return AuthResult::Failure;
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

                return AuthResult::Success { user, session };
            }
            Err(e) => {
                warn!("Failed to get user from db: {}", e);
                return AuthResult::Failure;
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

pub struct UserEx(pub Arc<User>);

impl Deref for UserEx {
    type Target = Arc<User>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for UserEx {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            use actix_web::error::ErrorUnauthorized;
            let session = SessionEx::from_request(&req, &mut actix_web::dev::Payload::None).await?;

            let user = req
                .app_data::<Data<UserManager>>()
                .unwrap()
                .get_user_by_session(&session)
                .map(|u| UserEx(u));

            user.ok_or(ErrorUnauthorized("access_denied"))
        })
    }
}
