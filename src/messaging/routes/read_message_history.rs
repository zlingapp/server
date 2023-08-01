use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorForbidden},
    get,
    web::{Path, Query},
    Error, HttpResponse,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::warn;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    auth::access_token::AccessToken,
    db::DB,
    messaging::message::{Message, MessageAuthor},
};

#[derive(Deserialize)]
pub struct MessageHistoryQuery {
    limit: Option<i64>,
    before: Option<DateTime<Utc>>,
    after: Option<DateTime<Utc>>,
}

const MAX_MESSAGE_LIMIT: i64 = 50;

#[get("/guilds/{guild_id}/channels/{channel_id}/messages")]
async fn read_message_history(
    db: DB,
    token: AccessToken,
    path: Path<(String, String)>,
    req: Query<MessageHistoryQuery>,
) -> Result<HttpResponse, Error> {
    let (guild_id, channel_id) = path.into_inner();

    let can_read = db
        .can_user_read_message_history_from(&token.user_id, &guild_id, &channel_id)
        .await
        .unwrap();

    if !can_read {
        return Err(ErrorForbidden("access_denied"));
    }

    let limit = req.limit.unwrap_or(MAX_MESSAGE_LIMIT);

    if limit <= 0 {
        return Err(ErrorBadRequest("invalid_limit"));
    }

    if limit > MAX_MESSAGE_LIMIT {
        return Err(ErrorBadRequest(format!(
            "max_limit_exceeds_{}",
            MAX_MESSAGE_LIMIT
        )));
    }

    let messages = sqlx::query!(
        r#"
        SELECT 
            messages.id, 
            messages.content, 
            messages.created_at,
            messages.attachments,
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
            AND messages.created_at < $3
            AND messages.created_at > $4
        )
        ORDER BY messages.created_at DESC 
        LIMIT $2
        "#,
        channel_id,
        limit,
        req.before.unwrap_or(Utc::now()).naive_utc(),
        req.after
            .map(|d| d.naive_utc())
            .unwrap_or(NaiveDateTime::from_timestamp_opt(0, 0).unwrap())
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
        .map(|record| {
            let attachments = match record.attachments.clone() {
                Some(some) => serde_json::from_value(some).ok(),
                None => None,
            };

            let message = Message {
                id: record.id.clone(),
                content: record.content.clone(),
                attachments,
                created_at: DateTime::<Utc>::from_utc(record.created_at, Utc),
                author: MessageAuthor {
                    id: record.author_id.clone(),
                    username: record.author_username.clone(),
                    avatar: record.author_avatar.clone(),
                },
            };

            serde_json::to_value(message).unwrap()
        })
        .collect();

    Ok(if messages.len() < limit as usize {
        HttpResponse::Ok()
    } else {
        HttpResponse::PartialContent()
    }
    .json(messages))
}
