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
    friends::dmchannel::{DMChannel, DMPath},
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
    let messages_deleted = sqlx::query!(
        "DELETE FROM messages WHERE id = $1 AND channel_id = $2 AND user_id = $3",
        message_path.message_id,
        channel.id,
        user.id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if messages_deleted == 0 {
        // if the channel id is not your own, you can't delete the message so
        // you get a 403

        // if the message is not your own, you can't delete the message so you
        // get a 403

        // if the message doesn't exist, we don't want to leak that information
        // anyways, so you get a 403
        err!(403)?;
    }

    pubsub
        .notify_dm_message_deleted(&channel.id, &user.id, &message_path.message_id)
        .await;

    // TODO: standardize ok responses as json here
    Ok(HttpResponse::Ok().finish())
}
