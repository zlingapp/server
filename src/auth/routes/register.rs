use std::sync::Arc;

use actix_web::{
    error::{ErrorBadRequest, ErrorConflict, ErrorInternalServerError},
    post,
    web::Json,
    Error, HttpResponse,
};
use lazy_static::lazy_static;
use log::warn;
use nanoid::nanoid;
use rand::Rng;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{auth::user::User, db::DB};

lazy_static! {
    pub static ref EMAIL_REGEX: regex::Regex = regex::Regex::new(r"(?i)^[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$").unwrap();
    pub static ref USERNAME_REGEX: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9!?._ -]{3,16}$").unwrap();
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// The email address of the user
    email: String,
    /// The password of the user
    password: String,
    /// The desired username of the user
    username: String,
}

/// Register
///
/// Register a new user account using an email address and password.
/// Does not log the user in automatically, please use the login endpoint for that
#[utoipa::path(
    responses(
        (status = CONFLICT, description = "User with that email already exists", example = "already_exists"),
        (status = BAD_REQUEST, description = "Invalid or malformed details"),
        (status = OK, description = "Registration successful", example = "success")
    ),
    tag = "identity"
)]
#[post("/auth/register")]
pub async fn register(db: DB, req: Json<RegisterRequest>) -> Result<HttpResponse, Error> {
    if !USERNAME_REGEX.is_match(&req.username) {
        return Err(ErrorBadRequest("invalid_username"));
    }

    if !EMAIL_REGEX.is_match(&req.email) {
        return Err(ErrorBadRequest("invalid_email"));
    }

    let user = Arc::new(User {
        id: nanoid!(),
        name: String::from_iter([&req.username, "#", &generate_discrim()]),
        email: Some(req.email.to_owned()),
        avatar: "https://placehold.co/32".to_owned(),
        bot: false,
    });

    match db.register_user(&user, &req.password).await {
        Ok(did_create) => {
            if !did_create {
                // user already exists
                return Err(ErrorConflict("already_exists"));
            }
            return Ok(HttpResponse::Ok().body("success"));
        }
        Err(e) => {
            warn!("Failed to create user: {}", e);
            return Err(ErrorInternalServerError("failed"));
        }
    }
}
pub fn generate_discrim() -> String {
    let mut rng = rand::thread_rng();

    String::from_iter([
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
    ])
}
