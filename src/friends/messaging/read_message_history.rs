use actix_web::{get, web::Query, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    auth::user::PublicUserInfo,
    db::DB,
    error::{macros::err, HResult},
    friends::dmchannel::{DMChannel, DMPath},
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

const MAX_MESSAGE_LIMIT: i64 = 50;

/// Read message history
///
/// Get the messages in the channel sent in between `after` and `before`, up to
/// a limit of `limit`. The maximum value of `limit` can be
#[utoipa::path(
    tag = "DMs",
    security(("token" = [])),
    params(MessageHistoryQuery, DMPath),
    responses(
        (status = OK, description = "Message listing succeeded, no more messages to retreive", body = Vec<Message>),
        (status = PARTIAL_CONTENT, description = "Message listing succeeded, but there are more messages beyond limit", body = Vec<Message>),
        (status = FORBIDDEN, description = "You are not friends with that user"),
        (status = BAD_REQUEST, description = "Invalid message limit")
    )
)]
#[get("/friends/{user_id}/messages")]
async fn read_message_history(
    db: DB,
    channel: DMChannel,
    req: Query<MessageHistoryQuery>,
) -> HResult<HttpResponse> {
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
            users.id AS "author_id"
        FROM messages, users 
        WHERE (
            messages.channel_id = $1 
            AND users.id = messages.user_id
            AND messages.created_at < $3
            AND messages.created_at > $4
        )
        ORDER BY messages.created_at DESC 
        LIMIT $2
        "#,
        channel.id,
        limit,
        req.before.unwrap_or(Utc::now()).naive_utc(),
        req.after.unwrap_or_default().naive_utc(), // unix epoch
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
