use actix_web::{
    error::{Error, ErrorForbidden},
    post,
    web::Json,
    HttpRequest,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{auth::{token::Token, access_token::AccessToken}, db::DB, util::use_display};

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
) -> Result<Json<ReissueResponse>, Error> {
    let refresh_token: Token = body.refresh_token.parse().map_err(|e| {
        use crate::auth::token::TokenParseError::*;
        match e {
            InvalidFormat => ErrorForbidden("access_denied"),
            Expired => ErrorForbidden("token_expired"),
        }
    })?;

    let user_agent = match req.headers().get("User-Agent") {
        Some(user_agent) => match user_agent.to_str() {
            Ok(user_agent) => user_agent,
            Err(_) => "Unknown",
        },
        None => "Unknown",
    };

    use crate::auth::token_issuing::IssueAccessTokenResult::*;
    match db.reissue_access_token(refresh_token, user_agent).await {
        Success {
            access_token,
            refresh_token,
        } => Ok(Json(ReissueResponse{
            access_token,
            refresh_token,
        })),
        Failure => Err(ErrorForbidden("access_denied")),
    }
}
