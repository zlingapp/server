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
use serde_json::{json, Value};

use crate::{
    auth::{
        perms::{can_user_read_message_history_from, can_user_send_message_in, is_user_in_guild},
        user::UserEx,
    },
    guilds::handlers::GuildIdQuery,
    realtime::consumer_manager::EventConsumerManager,
    DB,
};

pub fn scope() -> actix_web::Scope {
    web::scope("/channels")
        .service(list_channels)
        .service(create_channel)
        .service(send_message)
        .service(message_history)
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
    user: UserEx,
    g_query: Query<GuildIdQuery>,
) -> Result<Json<Vec<ChannelInfo>>, Error> {
    let user_in_guild = is_user_in_guild(&db, &user.id, &g_query.id)
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
    user: UserEx,
    query: Json<CreateChannelRequest>,
) -> Result<HttpResponse, Error> {
    let user_in_guild = is_user_in_guild(&db, &user.id, &query.guild_id)
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
    req: Json<SendMessageRequest>,
    ecm: Data<EventConsumerManager>,
) -> Result<HttpResponse, Error> {
    if req.content.len() > 2000 {
        return Err(ErrorBadRequest("content_too_long"));
    }

    let can_send = can_user_send_message_in(&db, &user.id, &req.channel_id)
        .await
        .unwrap();
    if !can_send {
        return Err(ErrorUnauthorized("access_denied"));
    }
    let message = sqlx::query!(
        r#"
        WITH message AS (
            INSERT INTO messages 
            (id, guild_id, channel_id, user_id, content) 
            VALUES ($1, $2, $3, $4, $5) RETURNING messages.id, messages.created_at
        ) 
        SELECT message.id, message.created_at, members.nickname AS "author_nickname" FROM message 
        LEFT JOIN members ON members.guild_id = $2 AND members.user_id = $4 
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
    })?;

    // tell people listening to this channel that there's a new message
    ecm.notify_of_new_message(
        &db,
        &user,
        &req.channel_id,
        &message.id,
        &req.content,
        &message.created_at,
        message.author_nickname
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "id": message.id,
        "created_at": message.created_at.to_string()
    })))
}

#[derive(Deserialize)]
pub struct MessageHistoryQuery {
    #[serde(rename = "c")]
    channel_id: String,
    #[serde(rename = "l")]
    limit: Option<i64>,
}

#[get("/history")]
async fn message_history(
    db: DB,
    user: UserEx,
    req: Query<MessageHistoryQuery>,
) -> Result<HttpResponse, Error> {
    let can_read = can_user_read_message_history_from(&db, &user.id, &req.channel_id)
        .await
        .unwrap();

    if !can_read {
        return Err(ErrorUnauthorized("access_denied"));
    }

    let limit = req.limit.unwrap_or(50);

    let messages = sqlx::query!(
        r#"
        SELECT 
            messages.id, 
            messages.content, 
            messages.created_at,
            users.name AS "author_username",
            users.avatar AS "author_avatar",
            users.id AS "author_id",
            members.nickname AS "author_nickname"
        FROM messages, members, users 
        WHERE (
            messages.channel_id = $1 
            AND messages.user_id = members.user_id 
            AND messages.guild_id = members.guild_id
            AND members.user_id = users.id 
        )
        ORDER BY created_at DESC 
        LIMIT $2
        "#,
        req.channel_id,
        limit
    )
    .fetch_all(db.as_ref())
    .await
    .map_err(|e| {
        warn!("failed to fetch message history: {}", e);
        ErrorInternalServerError("fetch_failed")
    })?;

    let messages: Vec<Value> = messages
        .iter()
        .rev()
        .map(|message| {
            json!({
                "id": message.id,
                "content": message.content,
                "created_at": message.created_at.to_string(),
                "author": {
                    "id": message.author_id,
                    "username": message.author_username,
                    "avatar": message.author_avatar,
                    "nickname": message.author_nickname
                }
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(messages))
}
