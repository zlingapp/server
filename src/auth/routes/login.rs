use actix_web::{get, post};
use actix_web::{error::ErrorUnauthorized, web::Json, Error, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::db::DB;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[post("/auth/login")]
pub async fn login(
    db: DB,
    creds: Json<LoginRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_agent = match req.headers().get("User-Agent") {
        Some(user_agent) => match user_agent.to_str() {
            Ok(user_agent) => user_agent,
            Err(_) => "Unknown",
        },
        None => "Unknown",
    };

    let auth_result = db
        .issue_refresh_token(&creds.email, &creds.password, user_agent)
        .await;

    use crate::auth::token_issuing::IssueRefreshTokenResult::*;
    match auth_result {
        Failure => Err(ErrorUnauthorized("access_denied")),
        Success {
            user,
            access_token,
            refresh_token,
        } => Ok(HttpResponse::Ok().json(json!({
            "user": user,
            "accessToken": access_token.to_string(),
            "refreshToken": refresh_token.to_string(),
        }))),
    }
}
