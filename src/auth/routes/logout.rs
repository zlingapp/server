use actix_web::{get, HttpResponse};

use crate::{auth::access_token::AccessToken, error::HResult};

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
pub async fn logout(_token: AccessToken) -> HResult<HttpResponse> {
    // todo: invalidate token
    Ok(HttpResponse::Ok().body("success"))
}
