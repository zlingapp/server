use actix_web::{get, web::Json};

use crate::auth::user::User;

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
pub async fn whoami(user: User) -> Json<User> {
    Json(user)
}
