use actix_web::{post, web::Json, HttpRequest};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::{access_token::AccessToken, token::Token},
    db::DB,
    error::{macros::err, HResult},
    util::use_display,
};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReissueRequest {
    refresh_token: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReissueResponse {
    #[serde(serialize_with = "use_display")]
    access_token: AccessToken,
    #[serde(serialize_with = "use_display")]
    refresh_token: Token,
}

/// Reissue Tokens
///
/// Reissue an access & refresh token pair using a valid refresh token. This
/// endpoint is used to get a new access token when the old one expires. Please
/// note that the old refresh token is invalidated.
#[utoipa::path(
    responses(
        (status = FORBIDDEN, description = "Invalid or expired refresh token", example = "access_denied"),
        (status = OK, description = "Renew successful", body = ReissueResponse)
    ),
    tag = "identity"
)]
#[post("/auth/reissue")]
pub async fn reissue(
    db: DB,
    body: Json<ReissueRequest>,
    req: HttpRequest,
) -> HResult<Json<ReissueResponse>> {
    let refresh_token: Token = body.refresh_token.parse()?;

    if refresh_token.is_bot() {
        // for bots, just issue a new access token without invalidating the old refresh token
        let access_token = AccessToken::new(refresh_token.user_id.clone());

        return Ok(Json(ReissueResponse {
            access_token,
            refresh_token,
        }));
    }

    let user_agent = match req.headers().get("User-Agent") {
        Some(user_agent) => user_agent.to_str().unwrap_or("Unknown"),
        None => "Unknown",
    };

    use crate::auth::token_issuing::IssueAccessTokenResult::*;
    match db.reissue_access_token(refresh_token, user_agent).await {
        Success {
            access_token,
            refresh_token,
        } => Ok(Json(ReissueResponse {
            access_token,
            refresh_token,
        })),
        Failure => err!(403)?,
    }
}
