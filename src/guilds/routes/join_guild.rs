use actix_web::{get, error::{ErrorInternalServerError, ErrorBadRequest}, web::Redirect, Responder};
use log::warn;

use crate::{db::DB, auth::user::UserEx, guilds::routes::GuildPath};

// todo: phase this out for invite system. btw, this is GET so people can go in their browser to join a guild
#[get("/guilds/{guild_id}/join")]
pub async fn join_guild(
    db: DB,
    user: UserEx,
    req: GuildPath,
) -> Result<impl Responder, actix_web::Error> {
    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2) 
            AND EXISTS (SELECT 1 FROM guilds WHERE guilds.id = $2)
        "#,
        user.id,
        req.guild_id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        warn!(
            "user {} failed to join guild {}: {}",
            user.id, req.guild_id, e
        );
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorBadRequest("join_invalid"));
    }

    // again, this is temporarily here so the browser redirects back to /
    Ok(Redirect::to("/").see_other())
}