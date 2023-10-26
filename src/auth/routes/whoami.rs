use actix_web::{get, web::Json};

use crate::auth::user::{User, UserEx};

/// Who am I?
///
/// Get the current user.
#[utoipa::path(
    responses(
        (status = OK, body = User)
    ),
    tag = "identity",
    security(("token" = []))
)]
#[get("/auth/whoami")]
pub async fn whoami(user: UserEx) -> Json<User> {
    Json(user.0)
}
