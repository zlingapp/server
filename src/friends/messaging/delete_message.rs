use actix_web::{
    delete,
    web::{Data, Path},
    HttpResponse,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    friends::dmchannel::{DMChannel,DMPath},
    realtime::pubsub::pubsub::PubSub,
};

#[derive(Deserialize, IntoParams)]
pub struct DeleteMessagePath {
    message_id: String,
}

/// Delete message
///
/// Deletes a specific message from a channel. You must either be the author of
/// the message or have the permission to manage messages in the channel.
#[utoipa::path(
    tag = "DMs",
    security(("token" = [])),
    params(DMPath, DeleteMessagePath),
    responses(
        (status = OK, description = "Message deleted successfully"),
        (status = FORBIDDEN, description = "You are not friends with that user"),
        (status = FORBIDDEN, description = "No permission to delete message")
    )
)]
#[delete("/friends/{user_id}/messages/{message_id}")]
pub async fn delete_message(
    db: DB,
    user: User,
    message_path: Path<DeleteMessagePath>,
    channel: DMChannel,
    pubsub: Data<PubSub>,
) -> HResult<HttpResponse> {
    if let Ok(message) = db.get_message(&channel.id, &message_path.message_id).await {
        if message.author.id != user.id {
            // No need to do any permission checks for a DM
            err!(403)?;
        }

        // delete the message from the db
        sqlx::query!("DELETE FROM messages WHERE id = $1", message.id)
            .execute(&db.pool)
            .await?;

        // tell clients that the message got deleted
        pubsub
            .notify_user_message_deletion(&channel.id, &message.id)
            .await;
    } else {
        // note: we don't want to return a 404 if the message doesn't exist, because
        // that would leak information about whether or not a message exists even if
        // the user doesn't have permission to view it. It doesn't really matter what
        // this returns
        err!(403)?;
    }

    Ok(HttpResponse::Ok().finish())
}
