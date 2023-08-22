use std::sync::Arc;

use actix_web::{
    error::{ErrorConflict, ErrorInternalServerError},
    post,
    web::Json,
    Error, HttpResponse,
};
use log::warn;
use nanoid::nanoid;
use rand::Rng;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{auth::user::User, db::DB};

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
        (status = CONFLICT, description = "User Already Exists", example = "already_exists"),
        (status = OK, description = "Registration Successful", example = "success")
    ),
    tag = "auth"
)]
#[post("/auth/register")]
pub async fn register(db: DB, req: Json<RegisterRequest>) -> Result<HttpResponse, Error> {
    let user = Arc::new(User {
        id: nanoid!(),
        name: String::from_iter([&req.username, "#", &generate_discrim()]),
        email: req.email.to_owned(),
        avatar: "https://placehold.co/32".to_owned(),
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
fn generate_discrim() -> String {
    let mut rng = rand::thread_rng();

    String::from_iter([
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
    ])
}
