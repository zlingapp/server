use actix_web::{
    post,
    web::{Data, Json, Path},
};
use chrono::{DateTime, Utc};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    friends::dmchannel::DMChannel,
    media::routes::upload::UploadedFileInfo,
    messaging::message::Message,
    realtime::pubsub::pubsub::PubSub,
};

#[derive(Deserialize, ToSchema)]
pub struct SendDMRequest {
    #[schema(example = "Hello from the API tester!")]
    content: Option<String>,
    attachments: Option<Vec<UploadedFileInfo>>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendDMResponse {
    #[schema(example = "K1vqjuY8OqU0VO7oJlGpY")]
    id: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize, IntoParams)]
pub struct SendDMPath {
    pub user_id: String,
}

/// Send a direct message
///
/// Sends a message in the DM Channel with the specified user, creating one if it doesnt exist
#[utoipa::path(
    tag = "DMs",
    security(("token" = [])),
    params(SendDMPath),
    responses(
        (status = OK, description = "Message sent", body = SendMessageResponse),
        (status = FORBIDDEN, description = "No permission to send message to user (not friends)"),
        (status = BAD_REQUEST, description = "Invalid message (content_too_long, missing_content)")
    )
)]
#[post("/friends/{user_id}/messages")]
async fn send_message(
    db: DB,
    user: User,
    req: Json<SendDMRequest>,
    channel: DMChannel,
    path: Path<SendDMPath>,
    pubsub: Data<PubSub>,
) -> HResult<Json<SendDMResponse>> {
    // get inner value
    let req = req.0;

    let is_content_empty = req.content.is_none() || req.content.as_ref().unwrap().is_empty();
    let are_attachments_empty =
        req.attachments.is_none() || req.attachments.as_ref().unwrap().is_empty();

    // ensure at least either content or attachments
    if is_content_empty && are_attachments_empty {
        err!(400, "Message cannot be empty with no attachments.")?;
    }

    // check content length
    if let Some(ref content) = req.content {
        if content.len() > 2000 {
            err!(400, "Message content is too long.")?;
        }
    }

    // permission check here
    let can_send = db.is_user_friend(&user.id, &path.user_id).await?;

    if !can_send {
        err!(403)?;
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
            (id, channel_id, user_id, content, attachments) 
            VALUES ($1, $2, $3, $4, $5) 
            RETURNING messages.id, messages.created_at
        ) 
        SELECT message.id, message.created_at, members.nickname AS "author_nickname" FROM message 
        LEFT JOIN members ON members.guild_id = $2 AND members.user_id = $4 
        "#,
        nanoid!(),
        channel.id,
        user.id,
        req.content,
        attachments
    )
    .fetch_one(&db.pool)
    .await?;

    let message = Message {
        id: record.id.clone(),
        content: if is_content_empty { None } else { req.content },
        attachments: if are_attachments_empty {
            None
        } else {
            req.attachments
        },
        author: user.into(),
        created_at: DateTime::<Utc>::from_naive_utc_and_offset(record.created_at, Utc),
    };

    // tell people listening to this channel that there's a new message
    pubsub
        .notify_user_of_new_message(&path.user_id, &message)
        .await;

    Ok(Json(SendDMResponse {
        id: message.id,
        created_at: message.created_at,
    }))
}
