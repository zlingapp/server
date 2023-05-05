use actix_web::{get, HttpResponse, Error, error::{ErrorUnauthorized, ErrorInternalServerError, ErrorBadRequest}, web::{Query, Path}};
use log::warn;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{db::DB, auth::user::UserEx};

#[derive(Deserialize)]
pub struct MessageHistoryQuery {
    limit: Option<i64>,
}

const MAX_MESSAGE_LIMIT: i64 = 50;

#[get("/guilds/{guild_id}/channels/{channel_id}/messages")]
async fn read_message_history(
    db: DB,
    user: UserEx,
    path: Path<(String, String)>,
    req: Query<MessageHistoryQuery>,
) -> Result<HttpResponse, Error> {
    let (guild_id, channel_id) = path.into_inner();

    let can_read = db.can_user_read_message_history_from(&guild_id, &user.id, &channel_id)
        .await
        .unwrap();

    if !can_read {
        return Err(ErrorUnauthorized("access_denied"));
    }

    let limit = req.limit.unwrap_or(50);

    if limit > MAX_MESSAGE_LIMIT {
        return Err(
            ErrorBadRequest(
                format!("max_limit_exceeds_{}", MAX_MESSAGE_LIMIT)
            )
        );
    }

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
        channel_id,
        limit
    )
    .fetch_all(&db.pool)
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
