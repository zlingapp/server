use actix_web::{get, Error, web::Json};

use crate::auth::user::{UserEx, User};

#[get("/auth/whoami")]
pub async fn whoami(user: UserEx) -> Result<Json<User>, Error> {
    Ok(Json(user.into()))
}