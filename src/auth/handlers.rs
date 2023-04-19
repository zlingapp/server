use actix_web::{
    get,
    web::{Data, Json, self},
    Error, HttpResponse, post,
};
use nanoid::nanoid;
use serde::Deserialize;
use serde_json::json;

use crate::auth::user::UserManager;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[post("/login")]
pub async fn login(um: Data<UserManager>, req: Json<LoginRequest>) -> Result<HttpResponse, Error> {
    use crate::auth::user::AuthResult::*;
    let res = um.authenticate(&req.email, &req.password);

    Ok(match res {
        Success(user) => HttpResponse::Ok().json(
            json!({
                "id": user.id,
                "username": user.name,
                "avatar": user.avatar,
                "session_token": nanoid!(64)
            })
        ),
        Failure => HttpResponse::Unauthorized().finish(),
    })
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    email: String,
    password: String,
    username: String,
}

#[post("/register")]
pub async fn register(um: Data<UserManager>, req: Json<RegisterRequest>) -> Result<HttpResponse, Error> {
    match um.register_user(&req.username, &req.email, &req.password) {
        Some(_) => Ok(HttpResponse::Ok().finish()),
        None => Ok(HttpResponse::Conflict().finish()),
    }
}

pub fn scope() -> actix_web::Scope {
    web::scope("/auth")
        .service(login)
        .service(register)
}
