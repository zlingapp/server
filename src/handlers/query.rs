use actix_web::{
    get,
    web::{Data, Json, Path},
};
use derive_more::{Display, Error};
use serde::Serialize;

use crate::Channels;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMemberInfo {
    identity: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryChannelReply {
    member_count: usize,
    members: Vec<ChannelMemberInfo>,
}

#[derive(Debug, Display, Error)]
pub enum QueryChannelError {
    #[display(fmt = "channel_not_found")]
    ChannelNotFound,
}

impl actix_web::error::ResponseError for QueryChannelError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        use QueryChannelError::*;
        match self {
            ChannelNotFound => StatusCode::NOT_FOUND,
        }
    }
}

#[get("/channel/{channel_id}")]
pub async fn query_channel(
    channels: Data<Channels>,
    path: Path<(String,)>,
) -> Result<Json<QueryChannelReply>, QueryChannelError> {
    let (channel_id,) = path.into_inner();
    let channel = channels.lock().unwrap().get(&channel_id).cloned();

    let channel = match channel {
        Some(existing) => existing,
        None => return Err(QueryChannelError::ChannelNotFound),
    };

    let reply;
    {
        let clients = channel.clients.lock().unwrap();
        reply = QueryChannelReply {
            member_count: clients.len(),
            members: clients
                .iter()
                .filter(|client| client.socket_session.read().unwrap().is_some())
                .map(|client| ChannelMemberInfo {
                    identity: client.identity.clone(),
                })
                .collect(),
        };
    }

    Ok(Json(reply))
}
