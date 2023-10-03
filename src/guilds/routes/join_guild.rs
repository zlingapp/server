#![allow(deprecated)]

use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError},
    get,
    web::{Redirect,Data},
    Responder,
};
use log::warn;

use crate::guilds::routes::GuildIdParams;
use crate::{auth::access_token::AccessToken, db::DB, guilds::routes::GuildPath,realtime::pubsub::consumer_manager::EventConsumerManager};

// todo: phase this out for invite system. btw, this is GET so people can go in
// their browser to join a guild

/// Join a Guild
///
/// This endpoint requires the user to not be in the guild already. For now, this
/// is a temporary endpoint until the invite system is implemented, so it has
/// been marked as deprecated.
///
/// Temporarily redirects to `/` on success. This is so the browser redirects
/// back to `/` after joining a guild, so invite links could sort of work.
#[utoipa::path(
    params(GuildIdParams),
    responses(
        (status = BAD_REQUEST, description = "Failed to join guild", example = "join_invalid"),
        (status = SEE_OTHER, description = "Joined guild successfully, redirect to /")
    ),
    tag = "guilds",
    security(("token" = []))
)]
#[deprecated]
#[get("/guilds/{guild_id}/join")]
pub async fn join_guild(
    db: DB,
    token: AccessToken,
    req: GuildPath,
    ecm: Data<EventConsumerManager>
) -> Result<impl Responder, actix_web::Error> {
    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2) 
            AND EXISTS (SELECT 1 FROM guilds WHERE guilds.id = $2)
        "#,
        token.user_id,
        req.guild_id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        warn!(
            "user {} failed to join guild {}: {}",
            token.user_id, req.guild_id, e
        );
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorBadRequest("join_invalid"));
    }
    ecm.notify_guild_member_list_update(&req.guild_id).await;
    // again, this is temporarily here so the browser redirects back to /
    Ok(Redirect::to("/").see_other())
}
