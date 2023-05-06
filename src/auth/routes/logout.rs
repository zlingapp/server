use actix_web::{get, Error, HttpResponse};

use crate::auth::token::TokenEx;

#[get("/auth/logout")]
pub async fn logout(_token: TokenEx) -> Result<HttpResponse, Error> {
    // todo: implement refresh token system
    // right now there is no way to invalidate a token!!! THIS IS BAD!!!
    Ok(HttpResponse::Ok().body("success"))
}
