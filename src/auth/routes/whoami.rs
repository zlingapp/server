use actix_web::{get, web::Json};

use crate::auth::user::{User, UserEx};

#[utoipa::path(
    get, path = "/auth/whoami",
    responses(
        (status = OK, body = User)
    )
)]
#[get("/auth/whoami")]
pub async fn whoami(user: UserEx) -> Json<User> {
    Json(user.0)
}
