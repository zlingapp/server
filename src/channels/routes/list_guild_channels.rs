use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    get,
    web::Json,
    Error,
};
use log::warn;
use serde::Serialize;

use crate::{
    auth::{access_token::AccessToken}, channels::channel::ChannelType, db::DB, guilds::routes::GuildPath,
};

#[derive(Serialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub r#type: ChannelType,
}

pub type ListChannelsResponse = Vec<ChannelInfo>;

#[get("/guilds/{guild_id}/channels")]
async fn list_guild_channels(
    db: DB,
    token: AccessToken,
    path: GuildPath,
) -> Result<Json<ListChannelsResponse>, Error> {
    let user_in_guild = db
        .is_user_in_guild(&token.user_id, &path.guild_id)
        .await
        .map_err(|e| {
            warn!(
                "failed to check if user {} is in guild {}: {}",
                token.user_id, path.guild_id, e
            );
            ErrorInternalServerError("")
        })?;

    if !user_in_guild {
        return Err(ErrorUnauthorized("access_denied"));
    }

    let channels = sqlx::query_as!(
        ChannelInfo,
        r#"SELECT id, name, type AS "type: _" FROM channels WHERE guild_id = $1"#,
        path.guild_id
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        warn!(
            "failed to retreive channels for guild {}: {}",
            path.guild_id, e
        );
        ErrorInternalServerError("")
    })?;

    Ok(Json(channels))
}
