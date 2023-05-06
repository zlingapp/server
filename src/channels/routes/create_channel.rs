use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized},
    post,
    web::Json,
    Error,
};
use lazy_static::lazy_static;
use log::warn;
use nanoid::nanoid;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    auth::token::TokenEx, channels::channel::ChannelType, db::DB, guilds::routes::GuildPath,
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
    token: TokenEx,
    req: Json<CreateChannelRequest>,
    path: GuildPath,
) -> Result<Json<CreateChannelResponse>, Error> {
    let user_in_guild = db
        .is_user_in_guild(&token.id, &path.guild_id)
        .await
        .map_err(|e| {
            warn!(
                "failed to check if user {} is in guild {}: {}",
                token.id, path.guild_id, e
            );
            ErrorInternalServerError("")
        })?;

    if !user_in_guild {
        return Err(ErrorUnauthorized("access_denied"));
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

    Ok(Json(CreateChannelResponse { id: channel_id }))
}
