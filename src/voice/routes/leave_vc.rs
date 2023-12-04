use actix_web::{get, web::Data, HttpResponse};

use crate::{
    error::HResult,
    voice::{client::VoiceClientEx, VoiceChannels, VoiceClients}, realtime::pubsub::pubsub::PubSub, db::DB,
};

/// Leave voice chat
///
/// Destroys server-side mediasoup objects, closes any ongoing voice
/// connections, revokes your credentials and notifies everyone you left. Call
/// this endpoint after gracefully `close()`-ing your clientside objects.
#[utoipa::path(
    tag = "voice",
    security(("voice" = [])),
    responses(
        (status = OK, description = "Disconnected successfully"),
    )
)]
#[get("/voice/leave")]
pub async fn leave_vc(
    client: VoiceClientEx,
    clients: Data<VoiceClients>,
    channels: Data<VoiceChannels>,
    pubsub: Data<PubSub>,
    db: DB
) -> HResult<HttpResponse> {
    client
        .channel
        .disconnect_client(&client, &clients, &channels)
        .await;
    pubsub.notify_voice_leave(&db.chan_to_guild(&client.channel.id).await.unwrap(), &client.user.clone().into(), &client.channel.id).await;
    Ok(HttpResponse::Ok().finish())
}
