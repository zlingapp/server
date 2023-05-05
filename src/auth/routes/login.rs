use actix_web::{post, web::{Data, Json}, HttpResponse, Error};
use serde::Deserialize;

use crate::auth::{SessionManager, routes::build_session_cookie};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[post("/auth/login")]
pub async fn login(sm: Data<SessionManager>, req: Json<LoginRequest>) -> Result<HttpResponse, Error> {
    use crate::auth::SessionAuthResult::*;
    let auth_result = sm.auth_new_session(&req.email, &req.password).await;

    Ok(match auth_result {
        Success { user, session } => {
            let session_cookie = build_session_cookie(&session).finish();
            HttpResponse::Ok().cookie(session_cookie).json(user)
        }
        Failure => HttpResponse::Unauthorized().body("access_denied"),
    })
}