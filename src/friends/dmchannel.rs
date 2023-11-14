use std::pin::Pin;

use actix_web::{web::Path, FromRequest};

use futures::Future;

use crate::db::DB;
use crate::error::{HResult, HandlerError, IntoHandlerErrorResult};

use crate::{auth::access_token::AccessToken, friends::messaging::send_message::SendDMPath};

pub struct DMChannel {
    pub id: String,
}

impl FromRequest for DMChannel {
    type Error = HandlerError;
    type Future = Pin<Box<dyn Future<Output = HResult<Self>>>>;
    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let to_id = &Path::<SendDMPath>::from_request(&req, &mut actix_web::dev::Payload::None)
                .await
                .or_err(400)?
                .into_inner()
                .user_id;
            let from_id = &AccessToken::from_request(&req, &mut actix_web::dev::Payload::None)
                .await?
                .user_id;
            Ok(Self {
                id: req
                    .app_data::<DB>()
                    .or_err(500)?
                    .get_dm_channel(from_id, to_id)
                    .await
                    .or_err(500)?,
            })
        })
    }
}
