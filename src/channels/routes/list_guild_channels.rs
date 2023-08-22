use actix_web::{
    error::{ErrorInternalServerError, ErrorForbidden},
    get,
    web::Json,
    Error,
};
use log::warn;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    auth::access_token::AccessToken, channels::channel::ChannelType, db::DB,
    guilds::routes::GuildPath,
};
use crate::guilds::routes::GuildIdParams;

#[derive(Serialize, ToSchema)]
pub struct ChannelInfo {
    #[schema(example = "jqNNyhSbOl1AwqCTMAZ2G")]
    pub id: String,
    #[schema(example = "memes")]
    pub name: String,
    pub r#type: ChannelType,
}

/// List Guild Channels
/// 
/// List all channels in a guild. This endpoint requires the user to be in the
/// guild of the channel, and have sufficient permissions to view the channel.
#[utoipa::path(
    params(GuildIdParams),
    responses(
        (status = FORBIDDEN, description = "No permission to view channel", example = "access_denied"),
        (status = OK, description = "Channel list", body = Vec<ChannelInfo>)
    ),
    tag = "channels",
    security(("token" = []))
)]
#[get("/guilds/{guild_id}/channels")]
async fn list_guild_channels(
    db: DB,
    token: AccessToken,
    path: GuildPath,
) -> Result<Json<Vec<ChannelInfo>>, Error> {
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
        return Err(ErrorForbidden("access_denied"));
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
