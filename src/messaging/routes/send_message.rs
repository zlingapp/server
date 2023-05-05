use actix_web::{
    error::ErrorBadRequest,
    post,
    web::{Data, Json, Path},
};
use nanoid::nanoid;
use serde::Deserialize;

use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    Error, HttpResponse,
};
use log::warn;
use serde_json::json;

use crate::{auth::user::UserEx, db::DB, realtime::pubsub::consumer_manager::EventConsumerManager};

#[derive(Deserialize)]
pub struct SendMessageRequest {
    content: String,
}

#[post("/guilds/{guild_id}/channels/{channel_id}/messages")]
async fn send_message(
    db: DB,
    user: UserEx,
    req: Json<SendMessageRequest>,
    path: Path<(String, String)>,
    ecm: Data<EventConsumerManager>,
) -> Result<HttpResponse, Error> {
    let (guild_id, channel_id) = path.into_inner();

    if req.content.len() > 2000 {
        return Err(ErrorBadRequest("content_too_long"));
    }

    let can_send = db.can_user_send_message_in(&guild_id, &user.id, &channel_id)
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
            VALUES ($1, $2, $3, $4, $5) 
            RETURNING messages.id, messages.created_at
        ) 
        SELECT message.id, message.created_at, members.nickname AS "author_nickname" FROM message 
        LEFT JOIN members ON members.guild_id = $2 AND members.user_id = $4 
        "#,
        nanoid!(),
        guild_id,
        channel_id,
        user.id,
        req.content
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to send message: {}", e);
        ErrorInternalServerError("send_failed")
    })?;

    // tell people listening to this channel that there's a new message
    ecm.notify_of_new_message(
        &user,
        &channel_id,
        &message.id,
        &req.content,
        &message.created_at,
        message.author_nickname,
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "id": message.id,
        "created_at": message.created_at.to_string()
    })))
}
