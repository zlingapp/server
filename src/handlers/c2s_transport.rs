use actix_web::{get, post, web::Json, ResponseError};
use derive_more::{Display, Error};
use mediasoup::{
    prelude::{DtlsParameters, IceCandidate, IceParameters},
    transport::Transport,
    webrtc_transport::WebRtcTransportRemoteParameters,
};
use serde::{Deserialize, Serialize};

use crate::{client::ClientEx, options::webrtc_transport_options};

// -------------- CREATE C2S TRANSPORT --------------

#[derive(Debug, Serialize)]
pub struct CreateSendTransportReply {
    id: String,
    ice_parameters: IceParameters,
    ice_candidates: Vec<IceCandidate>,
    dtls_parameters: DtlsParameters,
}

#[derive(Debug, Display, Error)]
pub struct CreateSendTransportError {}
impl ResponseError for CreateSendTransportError {}

pub type CreateSendTransportResponse =
    Result<Json<CreateSendTransportReply>, CreateSendTransportError>;

#[get("/transport/c2s/create")]
pub async fn create_c2s_transport(client: ClientEx) -> CreateSendTransportResponse {
    let transport = client
        .channel
        .router
        .create_webrtc_transport(webrtc_transport_options())
        .await
        .map_err(|_| CreateSendTransportError {})?;

    let reply = CreateSendTransportReply {
        id: transport.id().to_string(),
        ice_parameters: transport.ice_parameters().clone(),
        ice_candidates: transport.ice_candidates().clone(),
        dtls_parameters: transport.dtls_parameters(),
    };

    client
        .c2s_transports
        .write()
        .unwrap()
        .insert(transport.id().to_string(), transport);

    Ok(Json(reply))
}

// -------------- CONNECT C2S TRANSPORT --------------

#[derive(Debug, Deserialize)]
pub struct ConnectSendTransportRequest {
    transport_id: String,
    dtls_parameters: DtlsParameters,
}

#[derive(Debug, Display, Error)]
pub struct ConnectSendTransportError {}
impl ResponseError for ConnectSendTransportError {}

pub type ConnectSendTransportResponse = Result<&'static str, ConnectSendTransportError>;

#[post("/transport/c2s/connect")]
pub async fn connect_c2s_transport(
    client: ClientEx,
    request: Json<ConnectSendTransportRequest>,
) -> ConnectSendTransportResponse {
    client
        .c2s_transports
        .read()
        .unwrap()
        .get(&request.transport_id)
        .ok_or(ConnectSendTransportError {})?
        .connect(WebRtcTransportRemoteParameters {
            dtls_parameters: request.dtls_parameters.clone(),
        })
        .await
        .map_err(|_| ConnectSendTransportError {})?;

    Ok("connected")
}
