use actix_web::{get, web::Data, HttpResponse};

use crate::{voice::{client::VoiceClientEx, VoiceChannels, VoiceClients}, error::HResult};

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
) -> HResult<HttpResponse> {
    client
        .channel
        .disconnect_client(&client, &clients, &channels)
        .await;
    Ok(HttpResponse::Ok().finish())
}
