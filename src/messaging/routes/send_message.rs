use actix_web::{
    error::ErrorBadRequest,
    post,
    web::{Data, Json, Path},
};
use chrono::{DateTime, Utc};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    Error,
};
use log::warn;
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::{
    auth::user::{PublicUserInfo, UserEx},
    db::DB,
    media::routes::upload::UploadedFileInfo,
    messaging::message::Message,
    realtime::pubsub::consumer_manager::EventConsumerManager,
};

#[derive(Deserialize, ToSchema)]
pub struct SendMessageRequest {
    #[schema(example = "Hello from the API tester!")]
    content: Option<String>,
    attachments: Option<Vec<UploadedFileInfo>>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    #[schema(example = "K1vqjuY8OqU0VO7oJlGpY")]
    id: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize, IntoParams)]
struct SendMessagePath {
    guild_id: String,
    channel_id: String,
}

/// Send message
/// 
/// Sends a message with text `content` and with optional attachments.
#[utoipa::path(
    tag = "messaging",
    security(("token" = [])),
    params(SendMessagePath),
    responses(
        (status = OK, description = "Message sent", body = SendMessageResponse),
        (status = FORBIDDEN, description = "No permission to send message in channel"),
        (status = BAD_REQUEST, description = "Invalid message (content_too_long, missing_content)")
    )
)]
#[post("/guilds/{guild_id}/channels/{channel_id}/messages")]
async fn send_message(
    db: DB,
    user: UserEx,
    req: Json<SendMessageRequest>,
    path: Path<SendMessagePath>,
    ecm: Data<EventConsumerManager>,
) -> Result<Json<SendMessageResponse>, Error> {
    // get inner value
    let req = req.0;

    let is_content_empty = req.content.is_none() || req.content.as_ref().unwrap().is_empty();
    let are_attachments_empty =
        req.attachments.is_none() || req.attachments.as_ref().unwrap().is_empty();

    // ensure at least either content or attachments
    if is_content_empty && are_attachments_empty {
        return Err(ErrorBadRequest("missing_content"));
    }

    // check content length
    if let Some(ref content) = req.content {
        if content.len() > 2000 {
            return Err(ErrorBadRequest("content_too_long"));
        }
    }

    // permission check here
    let can_send = db
        .can_user_send_message_in(&user.id, &path.guild_id, &path.channel_id)
        .await
        .unwrap();

    if !can_send {
        return Err(ErrorForbidden("access_denied"));
    }

    // serialize attachments list back to json
    let attachments = match req.attachments {
        Some(ref atts) => serde_json::to_value(atts).unwrap(),
        None => Value::Null,
    };

    let record = sqlx::query!(
        r#"
        WITH message AS (
            INSERT INTO messages 
            (id, guild_id, channel_id, user_id, content, attachments) 
            VALUES ($1, $2, $3, $4, $5, $6) 
            RETURNING messages.id, messages.created_at
        ) 
        SELECT message.id, message.created_at, members.nickname AS "author_nickname" FROM message 
        LEFT JOIN members ON members.guild_id = $2 AND members.user_id = $4 
        "#,
        nanoid!(),
        path.guild_id,
        path.channel_id,
        user.id,
        req.content,
        attachments
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to send message: {}", e);
        ErrorInternalServerError("send_failed")
    })?;

    let message = Message {
        id: record.id.clone(),
        content: if is_content_empty { None } else { req.content },
        attachments: if are_attachments_empty {
            None
        } else {
            req.attachments
        },
        author: PublicUserInfo {
            id: user.0.id,
            username: user.0.name,
            avatar: user.0.avatar,
        },
        created_at: DateTime::<Utc>::from_utc(record.created_at, Utc),
    };

    // tell people listening to this channel that there's a new message
    ecm.notify_of_new_message(&path.channel_id, &message).await;

    Ok(Json(SendMessageResponse {
        id: message.id,
        created_at: message.created_at,
    }))
}
