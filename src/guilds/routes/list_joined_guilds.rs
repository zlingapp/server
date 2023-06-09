use actix_web::{error::ErrorInternalServerError, get, web::Json};
use log::warn;
use serde::Serialize;

use crate::{auth::access_token::AccessToken, db::DB};

#[derive(Serialize)]
pub struct GuildInfo {
    id: String,
    name: String,
    icon: Option<String>,
}

#[get("/guilds")]
pub async fn list_joined_guilds(
    db: DB,
    token: AccessToken,
) -> Result<Json<Vec<GuildInfo>>, actix_web::Error> {
    let guilds_list = sqlx::query_as!(
        GuildInfo,
        r#"
            SELECT guilds.id, guilds.name, guilds.icon FROM members, guilds 
            WHERE members.user_id = $1 AND members.guild_id = guilds.id
        "#,
        token.user_id
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to list guilds for user {}: {}", token.user_id, e);
        ErrorInternalServerError("failed")
    })?;

    Ok(Json(guilds_list))
}
