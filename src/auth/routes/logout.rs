use actix_web::{get, Error, HttpResponse};

use crate::auth::access_token::AccessToken;

#[get("/auth/logout")]
pub async fn logout(_token: AccessToken) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("success"))
}
