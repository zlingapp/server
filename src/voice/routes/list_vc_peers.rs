use actix_web::{get, web::Json};
use derive_more::{Display, Error};
use serde::Serialize;

use crate::{voice::client::VoiceClientEx, auth::user::PublicUserInfo};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMemberInfo {
    identity: String,
    producers: Vec<String>,
    user: PublicUserInfo,
}

#[derive(Debug, Display, Error)]
#[display(rename_all = "snake_case")]
pub enum QueryChannelError {}
impl actix_web::error::ResponseError for QueryChannelError {}

#[get("/voice/peers")]
pub async fn list_vc_peers(
    client: VoiceClientEx,
) -> Result<Json<Vec<ChannelMemberInfo>>, QueryChannelError> {
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

    Ok(Json(reply))
}
