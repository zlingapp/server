use std::pin::Pin;

use std::ops::Deref;

use actix_web::error::ErrorInternalServerError;
use actix_web::FromRequest;

use futures::Future;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{auth::access_token::AccessToken, db::DB};

/// User Account Information
// the email field shouldn't be known by users other than this user for privacy reasons
// only send this User struct to the user it references
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct User {
    // do not store sensitive information in here
    // this may be sent directly to the client
    // one example is the /whoami endpoint
    #[schema(example = "xoKM4W7NDqHjK_V0g9s3y")]
    pub id: String,
    #[schema(example = "someone#1234")]
    pub name: String,
    #[schema(example = "/api/media/9ybevZcdBh-3Z2KRLBidT/avatar.png")]
    pub avatar: String,
    #[schema(example = "someone@example.com")]
    pub email: Option<String>,
    #[schema(example = "false")]
    pub bot: bool,
}

// helper struct for representing user info to other users
// the fields here should not be sensitive info, eg. email
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicUserInfo {
    #[schema(example = "xoKM4W7NDqHjK_V0g9s3y")]
    pub id: String,
    #[schema(example = "someone#1234")]
    pub username: String,
    #[schema(example = "/api/media/9ybevZcdBh-3Z2KRLBidT/avatar.png")]
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

pub struct UserEx(pub User);

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

            user.ok_or(ErrorUnauthorized("authentication_required"))
        })
    }
}
