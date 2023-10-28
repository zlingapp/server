use actix_web::{post, web::Json, HttpResponse};
use log::warn;
use serde::Deserialize;

use crate::{
    auth::access_token::AccessToken,
    db::DB,
    error::{macros::err, HResult},
};

#[derive(Deserialize)]
pub struct SetAvatarRequest {
    avatar: String,
}

#[post("/settings/avatar")]
pub async fn set_avatar(
    token: AccessToken,
    req: Json<SetAvatarRequest>,
    db: DB,
) -> HResult<HttpResponse> {
    // todo: better check here
    if !req.avatar.starts_with("/media/") {
        err!(400, "Invalid avatar URL.")?;
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
    .await?;

    if result.rows_affected() != 1 {
        warn!("avatar set nothing");
        err!()?;
    }

    Ok(HttpResponse::Ok().finish())
}
