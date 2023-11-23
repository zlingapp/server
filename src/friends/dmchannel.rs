use std::pin::Pin;

use actix_web::dev::Payload;
use actix_web::{web::Path, FromRequest};

use crate::db::DB;
use crate::error::macros::err;
use crate::error::{HResult, HandlerError, IntoHandlerErrorResult};
use futures::Future;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::access_token::AccessToken;

#[derive(Deserialize, IntoParams)]
pub struct DMPath {
    pub user_id: String,
}

pub struct DMChannel {
    pub id: String,
    pub to_user_id: String,
}

impl FromRequest for DMChannel {
    type Error = HandlerError;
    type Future = Pin<Box<dyn Future<Output = HResult<Self>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut Payload,
    ) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let to_id = &Path::<DMPath>::from_request(&req, &mut Payload::None)
                .await
                .or_err(400)?
                .user_id;

            let from_id = &AccessToken::from_request(&req, &mut Payload::None)
                .await?
                .user_id;

            let db = req.app_data::<DB>().or_err(500)?;

            if !db.is_user_friend(from_id, to_id).await? {
                err!(403, "You cannot message this user as you are not their friend.")?;
            }

            Ok(Self {
                id: db.get_dm_channel_id(from_id, to_id).await.or_err(500)?,
                to_user_id: to_id.clone(),
            })
        })
    }
}
