use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError},
    post,
    web::Json,
    Error, HttpResponse,
};
use log::warn;
use serde::Deserialize;

use crate::{auth::access_token::AccessToken, db::DB};

#[derive(Deserialize)]
pub struct SetAvatarRequest {
    avatar: String,
}

#[post("/settings/avatar")]
pub async fn set_avatar(
    token: AccessToken,
    req: Json<SetAvatarRequest>,
    db: DB,
) -> Result<HttpResponse, Error> {
    // todo: better check here
    if !req.avatar.starts_with("/api/media/") {
        return Err(ErrorBadRequest("invalid_avatar"));
    }

    // alter avatar in db
    let result = sqlx::query!(
        r#"
            UPDATE users
            SET avatar = $1
            WHERE id = $2
        "#,
        req.avatar,
        token.user_id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to set avatar: {}", e);
        ErrorInternalServerError("")
    })?;

    if result.rows_affected() != 1 {
        warn!("avatar set nothing");
        return Err(ErrorInternalServerError(""));
    }

    Ok(HttpResponse::Ok().finish())
}
