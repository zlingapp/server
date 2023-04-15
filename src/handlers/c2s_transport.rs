use actix_web::{get, post, web::Json, ResponseError, http::header::TE};
use derive_more::{Display, Error};
use log::{info, error};
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
// #[display(fmt = "create_failed")]
pub enum CreateSendTransportError {
    #[display(fmt = "transport_already_exists")]
    TransportAlreadyExists,
    #[display(fmt = "transport_create_failed")]
    TransportCreateFailed,
}
impl ResponseError for CreateSendTransportError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use CreateSendTransportError::*;
        use actix_web::http::StatusCode;
        match self {
            TransportAlreadyExists => StatusCode::CONFLICT,
            TransportCreateFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type CreateSendTransportResponse =
    Result<Json<CreateSendTransportReply>, CreateSendTransportError>;

#[get("/transport/c2s/create")]
pub async fn create_c2s_transport(client: ClientEx) -> CreateSendTransportResponse {
    use CreateSendTransportError::*;

    if client.c2s_transport.read().unwrap().is_some() {
        return Err(TransportAlreadyExists);
    }

    let transport = client
        .channel
        .router
        .create_webrtc_transport(webrtc_transport_options())
        .await
        .map_err(|e| {
            error!("client[{}]: c2s transport create failed: {}", client.identity, e);
            TransportCreateFailed
        })?;

    let reply = CreateSendTransportReply {
        id: transport.id().to_string(),
        ice_parameters: transport.ice_parameters().clone(),
        ice_candidates: transport.ice_candidates().clone(),
        dtls_parameters: transport.dtls_parameters(),
    };

    info!("client[{}]: c2s transport created, id: {}", client.identity, transport.id());

    *client.c2s_transport.write().unwrap() = Some(transport);

    Ok(Json(reply))
}

// -------------- CONNECT C2S TRANSPORT --------------

#[derive(Debug, Deserialize)]
pub struct ConnectSendTransportRequest {
    dtls_parameters: DtlsParameters,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "connect_failed")]
pub struct ConnectSendTransportError {}
impl ResponseError for ConnectSendTransportError {}

pub type ConnectSendTransportResponse = Result<&'static str, ConnectSendTransportError>;

#[post("/transport/c2s/connect")]
pub async fn connect_c2s_transport(
    client: ClientEx,
    request: Json<ConnectSendTransportRequest>,
) -> ConnectSendTransportResponse {
    // read lock here is held across an await: problem??? :troll:
    client
        .c2s_transport
        .read()
        .unwrap()
        .as_ref()
        .ok_or(ConnectSendTransportError {})?
        .connect(WebRtcTransportRemoteParameters {
            dtls_parameters: request.dtls_parameters.clone(),
        })
        .await
        .map_err(|e| {
            error!("client[{}]: c2s connect failed: {}", client.identity, e);
            ConnectSendTransportError {}
        })?;

    info!("client[{}]: c2s transport connected", client.identity);

    Ok("connected")
}
