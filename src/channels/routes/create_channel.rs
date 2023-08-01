use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorForbidden},
    post,
    web::{Json, Data},
    Error,
};
use lazy_static::lazy_static;
use log::warn;
use nanoid::nanoid;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    auth::access_token::AccessToken, channels::channel::ChannelType, db::DB, guilds::routes::GuildPath, realtime::pubsub::consumer_manager::EventConsumerManager,
};

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    name: String,
    r#type: ChannelType,
}

#[derive(Serialize)]
pub struct CreateChannelResponse {
    id: String,
}

lazy_static! {
    // TODO: consider that this app becomes unusable for literally anyone who does not speak english
    // rework this regex to support other languages
    static ref CHANNEL_NAME_REGEX: Regex = Regex::new(r"^[\x20-\x7E]{1,16}$").unwrap();
}

#[post("/guilds/{guild_id}/channels")]
async fn create_channel(
    db: DB,
    token: AccessToken,
    req: Json<CreateChannelRequest>,
    path: GuildPath,
    ecm: Data<EventConsumerManager>,
) -> Result<Json<CreateChannelResponse>, Error> {
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

    if req.name.trim().is_empty() || !CHANNEL_NAME_REGEX.is_match(&req.name) {
        return Err(ErrorBadRequest("invalid_name"));
    }

    let channel_id = sqlx::query!(
        r#"INSERT INTO channels (guild_id, id, name, type) VALUES ($1, $2, $3, $4) RETURNING id"#,
        path.guild_id,
        nanoid!(),
        req.name,
        req.r#type as ChannelType
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to create channel: {}", e);
        ErrorInternalServerError("")
    })?
    .id;

    ecm.notify_guild_channel_list_update(&path.guild_id).await;

    Ok(Json(CreateChannelResponse { id: channel_id }))
}
