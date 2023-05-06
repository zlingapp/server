use actix_web::{
    error::{Error, ErrorUnauthorized},
    post,
    web::Json,
    HttpRequest, HttpResponse,
};
use serde::Deserialize;
use serde_json::json;

use crate::{auth::token::Token, db::DB};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReissueRequest {
    refresh_token: String,
}

#[post("/auth/reissue")]
pub async fn reissue(
    db: DB,
    body: Json<ReissueRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let refresh_token: Token = body.refresh_token.parse().map_err(|e| {
        use crate::auth::token::TokenParseError::*;
        match e {
            InvalidFormat => ErrorUnauthorized("access_denied"),
            Expired => ErrorUnauthorized("token_expired"),
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
        } => Ok(HttpResponse::Ok().json(json!({
            "accessToken": access_token.to_string(),
            "refreshToken": refresh_token.to_string(),
        }))),
        Failure => Err(ErrorUnauthorized("access_denied")),
    }
}
