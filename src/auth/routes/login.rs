use actix_web::{
    error::ErrorUnauthorized,
    post,
    web::{Json},
    Error, HttpResponse,
};
use log::warn;
use serde::Deserialize;
use serde_json::json;
use sqlx::query;

use crate::{
    auth::{token::Token, user::User},
    crypto,
    db::DB,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

pub enum AuthResult {
    Failure,
    Success { user: User, token: Token },
}

pub async fn do_username_password_login(db: DB, email: &str, password: &str) -> AuthResult {
    let user = query!(
        "SELECT id, name, email, avatar, password FROM users WHERE email = $1",
        email
    )
    .fetch_one(&db.pool)
    .await;

    match user {
        Ok(record) => {
            if !crypto::verify(password, &record.password) {
                return AuthResult::Failure;
            }

            let user = User {
                id: record.id.clone(),
                name: record.name,
                email: record.email,
                avatar: record.avatar,
            };

            let token = Token::new(record.id);
            return AuthResult::Success { user, token };
        }
        Err(e) => {
            warn!("Failed to get user from db: {}", e);
            return AuthResult::Failure;
        }
    }
}

#[post("/auth/login")]
pub async fn login(db: DB, req: Json<LoginRequest>) -> Result<HttpResponse, Error> {
    use AuthResult::*;
    let auth_result = do_username_password_login(db, &req.email, &req.password).await;

    match auth_result {
        Success { user, token } => {
            let token = token.to_string();
            Ok(HttpResponse::Ok().json(json!({ "user": user, "token": token })))
        }
        Failure => Err(ErrorUnauthorized("access_denied")),
    }
}
