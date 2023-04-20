use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, RwLock},
};

use std::ops::Deref;

use actix_web::{web::Data, FromRequest};
use futures::Future;
use nanoid::nanoid;
use rand::Rng;

use crate::util::constant_time_compare;

pub type UserId = String;
pub type SessionToken = String;

pub struct User {
    pub id: UserId,
    pub name: String,
    pub avatar: String,
    pub email: String,
    pub password: String, // todo: remove me! replace with db lookup!
    pub sessions: RwLock<Vec<SessionToken>>,
}

pub struct UserManager {
    // todo: use a database for this
    users: RwLock<HashMap<UserId, Arc<User>>>,
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
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    // todo: implement db stuff
    pub fn register_user(&self, name: &str, email: &str, password: &str) -> Option<Arc<User>> {
        if self
            .users
            .read()
            .unwrap()
            .values()
            .any(|u| u.email == email || u.name == name)
        {
            return None;
        }

        // todo: validate some regex here...
        let user = Arc::new(User {
            id: nanoid!(),
            name: String::from_iter([name, "#", &generate_discrim()]),
            email: email.to_owned(),
            avatar: "https://placehold.co/32".to_owned(),
            password: password.to_owned(),
            sessions: RwLock::new(Vec::new()),
        });

        self.users
            .write()
            .unwrap()
            .insert(user.id.clone(), user.clone());

        Some(user)
    }

    pub fn get_user_by_id(&self, id: &str) -> Option<Arc<User>> {
        self.users.read().unwrap().get(id).cloned()
    }

    pub fn get_user_by_name(&self, username: &str) -> Option<Arc<User>> {
        self.users
            .read()
            .unwrap()
            .values()
            .find(|user| user.name == username)
            .cloned()
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<Arc<User>> {
        self.users
            .read()
            .unwrap()
            .values()
            .find(|user| user.email == email)
            .cloned()
    }

    pub fn auth_new_session(&self, email: &str, password: &str) -> AuthResult {
        if let Some(user) = self.get_user_by_email(email) {
            // todo: replace with db lookup!
            if constant_time_compare(&user.password, password) {
                let session = nanoid!(64);

                user.sessions.write().unwrap().push(session.clone());
                self.sessions
                    .write()
                    .unwrap()
                    .insert(session.clone(), user.clone());

                return AuthResult::Success { user, session };
            }
        }
        AuthResult::Failure
    }

    pub fn get_user_by_session(&self, session: &str) -> Option<Arc<User>> {
        self.sessions.read().unwrap().get(session).cloned()
    }

    pub fn erase_session(&self, user: &User, session: &str) {
        user.sessions.write().unwrap().retain(|s| s != session);
        self.sessions.write().unwrap().remove(session);
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
