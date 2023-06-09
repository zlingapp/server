use std::pin::Pin;

use std::ops::Deref;

use actix_web::error::ErrorInternalServerError;
use actix_web::FromRequest;

use futures::Future;
use serde::Serialize;

use crate::{auth::access_token::AccessToken, db::DB};

pub type UserId = String;

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

// helper struct for representing user info to other users
// the fields here should not be sensitive info, eg. email
#[derive(Serialize)]
pub struct PublicUserInfo {
    pub id: UserId,
    pub username: String,
    pub avatar: String,
}

impl From<User> for PublicUserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.name,
            avatar: user.avatar,
        }
    }
}

pub struct UserEx(User);

impl Deref for UserEx {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<User> for UserEx {
    fn into(self) -> User {
        self.0
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
            let token = AccessToken::from_request(&req, &mut actix_web::dev::Payload::None).await?;

            let user = req
                .app_data::<DB>()
                .unwrap()
                .get_user_by_id(&token.user_id)
                .await
                .map_err(|e| {
                    log::error!("failed to get user from db: {}", e);
                    ErrorInternalServerError("")
                })?
                .map(|u| UserEx(u));

            user.ok_or(ErrorUnauthorized("access_denied"))
        })
    }
}
