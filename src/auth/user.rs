use std::{
    pin::Pin,
    sync::{Arc},
};

use std::ops::Deref;

use actix_web::{web::Data, FromRequest};

use futures::Future;
use serde::Serialize;

use crate::auth::{SessionEx, SessionManager};

pub type UserId = String;
pub type SessionToken = String;

/// struct containing user info
/// the email field shouldn't be known by users other than this user for privacy reasons
/// only send this User struct to the user it references
#[derive(Debug, Clone, Serialize)]
pub struct User {
    // do not store sensitive information in here
    // this may be sent directly to the client
    // one example is the /whoami endpoint
    pub id: UserId,
    pub name: String,
    pub avatar: String,
    pub email: String,
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
                .app_data::<Data<SessionManager>>()
                .unwrap()
                .get_user_by_session(&session)
                .map(|u| UserEx(u));

            user.ok_or(ErrorUnauthorized("access_denied"))
        })
    }
}