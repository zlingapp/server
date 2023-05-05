use actix_web::{get, web::Data, Error, HttpResponse};

use crate::voice::{client::VoiceClientEx, VoiceChannels, VoiceClients};

#[get("/voice/leave")]
pub async fn leave_vc(
    client: VoiceClientEx,
    clients: Data<VoiceClients>,
    channels: Data<VoiceChannels>,
) -> Result<HttpResponse, Error> {
    client
        .channel
        .disconnect_client(&client, &clients, &channels)
        .await;
    Ok(HttpResponse::Ok().finish())
}
