use actix_web::post;
use actix_web::{error::ErrorForbidden, web::Json, Error, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::access_token::AccessToken;
use crate::auth::token::Token;
use crate::auth::user::User;
use crate::db::DB;
use crate::util::use_display;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "someone@example.com")]
    email: String,
    #[schema(example = "hunter2")]
    password: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponese {
    user: User,
    #[serde(serialize_with = "use_display")]
    access_token: AccessToken,
    #[serde(serialize_with = "use_display")]
    refresh_token: Token,
}

/// Log in
///
/// Log in using an email address and password and get an access token and refresh token
/// as well as a new `User` object. You may use the access token to authenticate yourself
/// for other endpoints, and the refresh token to get a new access token when the old one
/// expires at the reissue endpoint.
#[utoipa::path(
    responses(
        (status = FORBIDDEN, description = "Invalid username or password", example = "access_denied"),
        (status = OK, description = "Login successful", body = LoginResponese)
    ),
    tag = "identity"
)]
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
        Failure => Err(ErrorForbidden("access_denied")),
        Success {
            user,
            access_token,
            refresh_token,
        } => Ok(HttpResponse::Ok().json(LoginResponese {
            user: user,
            access_token: access_token,
            refresh_token: refresh_token,
        })),
    }
}
