use actix_web::{
    cookie::{Cookie, CookieBuilder, SameSite},
    get, post,
    web::{self, Data, Json},
    Error, HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;

use crate::auth::user::{SessionEx, UserEx, UserManager};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

fn build_session_cookie(session: &str) -> CookieBuilder {
    Cookie::build("Session", session)
        // only allow https
        .secure(true)
        // disallow js access
        .http_only(true)
        // send on all api calls
        .path("/api")
        // allow cookie to be sent on anywhere on the same domain, even if the user is coming from another site
        .same_site(SameSite::Lax)
}

#[post("/login")]
pub async fn login(um: Data<UserManager>, req: Json<LoginRequest>) -> Result<HttpResponse, Error> {
    use crate::auth::user::AuthResult::*;
    let auth_result = um.auth_new_session(&req.email, &req.password);

    Ok(match auth_result {
        Success { user, session } => {
            let session_cookie = build_session_cookie(&session).finish();

            HttpResponse::Ok().cookie(session_cookie).json(json!({
                "id": user.id,
                "username": user.name,
                "avatar": user.avatar,
            }))
        }
        Failure => HttpResponse::Unauthorized().body("access_denied"),
    })
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    email: String,
    password: String,
    username: String,
}

#[post("/register")]
pub async fn register(
    um: Data<UserManager>,
    req: Json<RegisterRequest>,
) -> Result<HttpResponse, Error> {
    match um.register_user(&req.username, &req.email, &req.password) {
        Some(_) => Ok(HttpResponse::Ok().body("success")),
        None => Ok(HttpResponse::Conflict().finish()),
    }
}

// Let a client get their own user info
#[get("/whoami")]
pub async fn whoami(user: UserEx) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(json!({
        "id": user.id,
        "username": user.name,
        "avatar": user.avatar,
        "email": user.email,
    })))
}

#[get("/logout")]
pub async fn logout(
    um: Data<UserManager>,
    user: UserEx,
    session: SessionEx,
) -> Result<HttpResponse, Error> {
    um.erase_session(&user, &session);

    let death_cookie = build_session_cookie("NO_SESSION_DELETE_ME")
        .expires(OffsetDateTime::from_unix_timestamp(0).unwrap())
        .finish();

    Ok(HttpResponse::Ok().cookie(death_cookie).body("success"))
}

pub fn scope() -> actix_web::Scope {
    web::scope("/auth")
        .service(login)
        .service(register)
        .service(whoami)
        .service(logout)
}
