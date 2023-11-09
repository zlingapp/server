use actix_web::{
    get,
    web::{Path, Query},
    HttpResponse,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    auth::{access_token::AccessToken, user::PublicUserInfo},
    db::DB,
    error::{macros::err, HResult},
    messaging::message::Message,
};

#[derive(Deserialize, IntoParams)]
pub struct MessageHistoryQuery {
    #[param(style = Form, minimum = 1, maximum = 50)]
    limit: Option<i64>,
    #[param(style = Form)]
    before: Option<DateTime<Utc>>,
    #[param(style = Form)]
    after: Option<DateTime<Utc>>,
}

#[derive(Deserialize, IntoParams)]
struct ReadHistoryPath {
    channel_id: String,
}

const MAX_MESSAGE_LIMIT: i64 = 50;

/// Read message history
///
/// Get the messages in the channel sent in between `after` and `before`, up to
/// a limit of `limit`. The maximum value of `limit` can be
#[utoipa::path(
    tag = "messaging",
    security(("token" = [])),
    params(MessageHistoryQuery, ReadHistoryPath),
    responses(
        (status = OK, description = "Message listing succeeded, no more messages to retreive", body = Vec<Message>),
        (status = PARTIAL_CONTENT, description = "Message listing succeeded, but there are more messages beyond limit", body = Vec<Message>),
        (status = FORBIDDEN, description = "No permission to read message history"),
        (status = BAD_REQUEST, description = "Invalid message limit")
    )
)]
#[get("/channels/{channel_id}/messages")]
async fn read_message_history(
    db: DB,
    token: AccessToken,
    path: Path<ReadHistoryPath>,
    req: Query<MessageHistoryQuery>,
) -> HResult<HttpResponse> {
    let can_read = db
        .can_user_read_message_history_from(&token.user_id, &path.channel_id)
        .await?;

    if !can_read {
        err!(403)?;
    }

    let limit = req.limit.unwrap_or(MAX_MESSAGE_LIMIT);

    if limit < 1 {
        err!(400, "Message limit cannot be less than 1.")?;
    }

    if limit > MAX_MESSAGE_LIMIT {
        err!(
            400,
            format!("Message limit cannot be more than {}.", MAX_MESSAGE_LIMIT)
        )?
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
            AND members.user_id = users.id
            AND messages.created_at < $3
            AND messages.created_at > $4
        )
        ORDER BY messages.created_at DESC 
        LIMIT $2
        "#,
        path.channel_id,
        limit,
        req.before.unwrap_or(Utc::now()).naive_utc(),
        req.after
            .map(|d| d.naive_utc())
            .unwrap_or(NaiveDateTime::from_timestamp_opt(0, 0).unwrap())
    )
    .fetch_all(&db.pool)
    .await?;

    let messages: Vec<Message> = messages
        .iter()
        .rev()
        .map(|record| {
            let attachments = match record.attachments.clone() {
                Some(some) => serde_json::from_value(some).ok(),
                None => None,
            };

            Message {
                id: record.id.clone(),
                content: record.content.clone(),
                attachments,
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(record.created_at, Utc),
                author: PublicUserInfo {
                    id: record.author_id.clone(),
                    username: record.author_username.clone(),
                    avatar: record.author_avatar.clone(),
                },
            }
        })
        .collect();

    Ok(if messages.len() < limit as usize {
        HttpResponse::Ok()
    } else {
        HttpResponse::PartialContent()
    }
    .json(messages))
}
