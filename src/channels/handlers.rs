/*

   - List channels in a guild
   - Get channel history
   - Send to a channel
   - Create a channel
   - Delete a channel

*/

use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized},
    get, post,
    web::{self, Data, Json, Query},
    Error, HttpResponse,
};
use log::warn;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    auth::user::{UserEx, UserManager},
    guilds::handlers::GuildIdQuery,
    DB,
};

pub fn scope() -> actix_web::Scope {
    web::scope("/channels")
        .service(list_channels)
        .service(create_channel)
}

#[derive(Copy, Clone, sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "channel_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}
#[derive(Serialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub r#type: ChannelType,
}

#[get("/list")]
async fn list_channels(
    db: DB,
    um: Data<UserManager>,
    user: UserEx,
    g_query: Query<GuildIdQuery>,
) -> Result<Json<Vec<ChannelInfo>>, Error> {
    let user_in_guild = um
        .is_user_in_guild(&user.id, &g_query.id)
        .await
        .map_err(|e| {
            warn!(
                "failed to check if user {} is in guild {}: {}",
                user.id, g_query.id, e
            );
            ErrorInternalServerError("")
        })?;

    if !user_in_guild {
        return Err(ErrorUnauthorized("access_denied"));
    }

    let channels = sqlx::query_as!(
        ChannelInfo,
        r#"SELECT id, type AS "type: _", name FROM channels WHERE guild_id = $1"#,
        g_query.id
    )
    .fetch_all(db.as_ref())
    .await
    .map_err(|e| {
        warn!(
            "failed to retreive channels for guild {}: {}",
            g_query.id, e
        );
        ErrorInternalServerError("")
    })?;

    Ok(Json(channels))
}

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    name: String,
    guild_id: String,
    r#type: ChannelType,
}

#[post("/create")]
async fn create_channel(
    db: DB,
    um: Data<UserManager>,
    user: UserEx,
    query: Json<CreateChannelRequest>,
) -> Result<HttpResponse, Error> {
    let user_in_guild = um
        .is_user_in_guild(&user.id, &query.guild_id)
        .await
        .map_err(|e| {
            warn!(
                "failed to check if user {} is in guild {}: {}",
                user.id, query.guild_id, e
            );
            ErrorInternalServerError("")
        })?;

    if !user_in_guild {
        warn!("user not in guild");
        return Err(ErrorUnauthorized("access_denied"));
    }

    let channel_id = sqlx::query!(
        r#"INSERT INTO channels (guild_id, id, name, type) VALUES ($1, $2, $3, $4) RETURNING id"#,
        query.guild_id,
        nanoid!(),
        query.name,
        query.r#type as ChannelType
    )
    .fetch_one(db.as_ref())
    .await
    .map_err(|e| {
        warn!("failed to create channel: {}", e);
        ErrorInternalServerError("")
    })?
    .id;

    Ok(HttpResponse::Created().json(json!({ "id": channel_id })))
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    guild_id: String,
    channel_id: String,
    content: String,
}

#[post("/send")]
async fn send_message(
    db: DB,
    user: UserEx,
    um: Data<UserManager>,
    req: Json<SendMessageRequest>,
) -> Result<HttpResponse, Error> {
    if req.content.len() > 2000 {
        return Err(ErrorBadRequest("content_too_long"));
    }

    let user_in_guild = um
        .is_user_in_guild(&user.id, &req.guild_id)
        .await
        .map_err(|e| {
            warn!(
                "failed to check if user {} is in guild {}: {}",
                user.id, req.guild_id, e
            );
            ErrorInternalServerError("")
        })?;

    if !user_in_guild {
        warn!("user not in guild");
        return Err(ErrorUnauthorized("access_denied"));
    }

    let message_id = sqlx::query!(
        r#"INSERT INTO messages 
        (id, guild_id, channel_id, user_id, content) 
        VALUES ($1, $2, $3, $4, $5) RETURNING id
        "#,
        nanoid!(),
        req.guild_id,
        req.channel_id,
        user.id,
        req.content
    )
    .fetch_one(db.as_ref())
    .await
    .map_err(|e| {
        warn!("failed to send message: {}", e);
        ErrorInternalServerError("send_failed")
    })?
    .id;

    Ok(HttpResponse::Ok().json(json!({ "id": message_id })))
}
