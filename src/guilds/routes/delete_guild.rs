use actix_web::{delete, error::{ErrorInternalServerError, ErrorUnauthorized}, HttpResponse};
use log::warn;

use crate::{db::DB, auth::user::UserEx, guilds::routes::GuildPath};

#[delete("/guilds/{guild_id}")]
pub async fn delete_guild(
    db: DB,
    user: UserEx,
    req: GuildPath,
) -> Result<HttpResponse, actix_web::Error> {
    let rows_affected = sqlx::query!(
        r#"
            DELETE FROM guilds WHERE id = $1 AND owner = $2
        "#,
        req.guild_id,
        user.id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to delete guild: {}", e);
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorUnauthorized("access_denied"));
    }

    Ok(HttpResponse::Ok().body("success"))
}