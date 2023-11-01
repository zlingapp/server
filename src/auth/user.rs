use std::pin::Pin;

use actix_web::FromRequest;

use futures::Future;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::access_token::AccessToken,
    db::DB,
    error::{HResult, HandlerError, IntoHandlerErrorResult},
};

/// User Account Information
// the email field shouldn't be known by users other than this user for privacy reasons
// only send this User struct to the user it references
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    // do not store sensitive information in here
    // this may be sent directly to the client
    // one example is the /whoami endpoint
    #[schema(example = "xoKM4W7NDqHjK_V0g9s3y")]
    pub id: String,
    #[schema(example = "someone#1234")]
    pub name: String,
    #[schema(example = "/media/9ybevZcdBh-3Z2KRLBidT/avatar.png")]
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
    #[schema(example = "/media/9ybevZcdBh-3Z2KRLBidT/avatar.png")]
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

impl FromRequest for User {
    type Error = HandlerError;
    type Future = Pin<Box<dyn Future<Output = HResult<Self>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let token = AccessToken::from_request(&req, &mut actix_web::dev::Payload::None).await?;

            req.app_data::<DB>()
                .or_err(500)?
                .get_user_by_id(&token.user_id)
                .await
                .or_err(500)?
                .or_err(401)
        })
    }
}
