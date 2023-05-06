use actix_web::{error::ErrorInternalServerError, get, web::Json};
use log::warn;
use serde::Serialize;

use crate::{auth::token::TokenEx, db::DB};

#[derive(Serialize)]
pub struct GuildNameAndId {
    id: String,
    name: String,
}

#[get("/guilds")]
pub async fn list_joined_guilds(
    db: DB,
    token: TokenEx,
) -> Result<Json<Vec<GuildNameAndId>>, actix_web::Error> {
    let guilds_list = sqlx::query_as!(
        GuildNameAndId,
        r#"
            SELECT members.guild_id AS "id", guilds.name FROM members, guilds 
            WHERE members.user_id = $1 AND members.guild_id = guilds.id
        "#,
        token.id
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to list guilds for user {}: {}", token.id, e);
        ErrorInternalServerError("failed")
    })?;

    Ok(Json(guilds_list))
}
