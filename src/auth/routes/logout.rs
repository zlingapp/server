use actix_web::{get, Error, HttpResponse};

use crate::auth::access_token::AccessToken;

/// Log out
/// 
/// Invalidate the current access token and log out.
#[utoipa::path(
    responses(
        (status = OK, description = "Logout successful", example = "success")
    ),
    tag = "identity",
    security(("token" = []))
)]
#[get("/auth/logout")]
pub async fn logout(_token: AccessToken) -> Result<HttpResponse, Error> {
    // todo: invalidate token
    Ok(HttpResponse::Ok().body("success"))
}
