use actix_web::{
    delete,
    web::{Data, Path},
    HttpResponse,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    auth::access_token::AccessToken,
    db::DB,
    error::{macros::err, HResult},
    realtime::pubsub::pubsub::PubSub,
};

#[derive(Deserialize, IntoParams)]
pub struct DeleteMessagePath {
    guild_id: String,
    channel_id: String,
    message_id: String,
}

/// Delete message
///
/// Deletes a specific message from a channel. You must either be the author of
/// the message or have the permission to manage messages in the channel.
#[utoipa::path(
    tag = "messaging",
    security(("token" = [])),
    params(DeleteMessagePath),
    responses(
        (status = OK, description = "Message deleted successfully"),
        (status = FORBIDDEN, description = "No permission to delete message")
    )
)]
#[delete("/guilds/{guild_id}/channels/{channel_id}/messages/{message_id}")]
pub async fn delete_message(
    db: DB,
    token: AccessToken,
    path: Path<DeleteMessagePath>,
    pubsub: Data<PubSub>,
) -> HResult<HttpResponse> {
    if let Ok(message) = db
        .get_message(&path.guild_id, &path.channel_id, &path.message_id)
        .await
    {
        // check if this user can view this message
        // yes this technically allows deleting a message if it's beyond your
        // message history, but we don't really care about that all too much
        // since it's a rare, niche case (basically, no one will notice or care)

        if !db
            .can_user_view_messages_in(&token.user_id, &path.guild_id, &path.channel_id)
            .await?
        {
            err!(403)?;
        }

        // author should always be able to delete their own messages
        if message.author.id != token.user_id {
            // otherwise, check if the user has permission to delete messages
            if !db
                .can_user_manage_messages(&token.user_id, &path.guild_id, &path.channel_id)
                .await?
            {
                err!(403)?;
            }
        }

        // delete the message from the db
        sqlx::query!("DELETE FROM messages WHERE id = $1", message.id)
            .execute(&db.pool)
            .await?;

        // tell clients that the message got deleted
        pubsub
            .notify_message_deleted(&path.channel_id, &message.id)
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
