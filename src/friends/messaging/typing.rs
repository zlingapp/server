use actix_web::{post, web::Data, HttpResponse};

use crate::{
    auth::user::User,
    error::HResult,
    friends::dmchannel::{DMChannel, DMPath},
    realtime::pubsub::pubsub::PubSub,
};

/// Typing
///
/// Notify the target channel that you are typing a message. Please call this
/// endpoint every `4s` while the user is still typing to maintain their status.
#[utoipa::path(
    tag = "DMs",
    security(("token" = [])),
    params(DMPath),
    responses(
        (status = OK, description = "Typing notification sent"),
        (status = FORBIDDEN, description = "You are not friends with that user"),
    )
)]
#[post("/friends/{user_id}/typing")]
pub async fn typing(channel: DMChannel, user: User, pubsub: Data<PubSub>) -> HResult<HttpResponse> {
    pubsub
        .send_dm_typing(&channel.to_user_id, &user.into())
        .await;

    Ok(HttpResponse::Ok().finish())
}
