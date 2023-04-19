use actix_web::{get, web::Json};
use derive_more::{Display, Error};
use serde::Serialize;

use crate::client::ClientEx;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMemberInfo {
    identity: String,
    producers: Vec<String>,
}

#[derive(Debug, Display, Error)]
#[display(rename_all = "snake_case")]
pub enum QueryChannelError {}
impl actix_web::error::ResponseError for QueryChannelError {}

#[get("/peers")]
pub async fn query_channel(
    client: ClientEx,
) -> Result<Json<Vec<ChannelMemberInfo>>, QueryChannelError> {
    let reply;
    {
        let clients = client.channel.clients.lock().unwrap();
        reply = clients
            .iter()
            .filter(|c| c.socket_session.read().unwrap().is_some())
            .filter(|c| c.identity != client.identity)
            .map(|c| ChannelMemberInfo {
                identity: c.identity.clone(),
                producers: c.producers.lock().unwrap().keys().cloned().collect(),
            })
            .collect();
    }

    Ok(Json(reply))
}
