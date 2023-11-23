use actix_web::{
    post,
    web::Data,
    HttpResponse,
};

use crate::{
    auth::user::User,
    friends::dmchannel::{DMChannel,DMPath},
    error::HResult,
    realtime::pubsub::pubsub::PubSub,
};

/// Typing
///
/// Notify the target channel that you are typing a message. Please call this
/// endpoint every `4s` while the user is still typing to maintain their status.
#[utoipa::path(
    tag = "messaging",
    security(("token" = [])),
    params(DMPath),
    responses(
        (status = OK, description = "Typing notification sent"),
        (status = FORBIDDEN, description = "No permission to type messages in channel"),
    )
)]
#[post("/channels/{channel_id}/typing")]
pub async fn typing(
    channel: DMChannel,
    user: User,
    pubsub: Data<PubSub>,
) -> HResult<HttpResponse> {

    pubsub.send_user_typing(&channel.to_user_id, &user).await;

    Ok(HttpResponse::Ok().finish())
}
