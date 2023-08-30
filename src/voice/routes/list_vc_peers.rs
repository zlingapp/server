use actix_web::{get, web::Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{voice::client::VoiceClientEx, auth::user::PublicUserInfo};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMemberInfo {
    identity: String,
    #[schema(value_type = Vec<uuid::Uuid>)]
    producers: Vec<String>,
    user: PublicUserInfo,
}

// #[derive(Debug, Display, Error)]
// #[display(rename_all = "snake_case")]
// pub enum QueryChannelError {}
// impl actix_web::error::ResponseError for QueryChannelError {}

/// List peers in channel
/// 
/// Gives you a list of everyone currently connected to the voice channel you're
/// in, and a list of their producers that you can consume.
/// 
/// This also gives you the underlying `User` information for a particular RTC
/// identity, which is useful for showing in the UI.
/// 
/// Please avoid repeated calling of this endpoint to update the peer list, as
/// the websocket should already notify you when peers leave and join with
/// equrivalent details about the peer.
#[utoipa::path(
    tag = "voice",
    security(("voice" = [])),
    responses(
        (status = OK, description = "List of peers"),
    )
)]
#[get("/voice/peers")]
pub async fn list_vc_peers(
    client: VoiceClientEx,
) -> Json<Vec<ChannelMemberInfo>> {
    let reply;
    {
        let clients = client.channel.clients.lock().unwrap();
        reply = clients
            .iter()
            .filter(|c| c.socket.read().unwrap().is_some())
            .filter(|c| c.identity != client.identity)
            .map(|c| ChannelMemberInfo {
                identity: c.identity.clone(),
                producers: c.producers.lock().unwrap().keys().cloned().collect(),
                user: PublicUserInfo::from(c.user.clone()),
            })
            .collect();
    }

    Json(reply)
}
