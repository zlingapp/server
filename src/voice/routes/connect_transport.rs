use actix_web::{
    post,
    web::{Json, Query},
    ResponseError,
};
use derive_more::{Display, Error};
use log::{error, info, warn};
use mediasoup::{prelude::DtlsParameters, webrtc_transport::WebRtcTransportRemoteParameters};
use serde::Deserialize;

use crate::voice::{client::VoiceClientEx, transport::TransportType, routes::create_transport::TransportTypeQuery};

/// This is the request body for the connect transport handler.
/// It contains the DTLS parameters of the remote peer, which are needed to connect the transport.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectTransportRequest {
    dtls_parameters: DtlsParameters,
}

/// If things go wrong when connecting a transport, this enum will be used to specify the error.
#[derive(Debug, Display, Error)]
#[display(rename_all = "snake_case")]
pub enum ConnectTransportError {
    TransportNotCreated,
    TransportConnectFailed,
}
impl ResponseError for ConnectTransportError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        use ConnectTransportError::*;
        match self {
            TransportNotCreated => StatusCode::BAD_REQUEST,
            TransportConnectFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type ConnectTransportResponse = Result<&'static str, ConnectTransportError>;

/// POST /transport/connect
///
///    Connects a transport to the remote peer. The type of transport is specified in the query string.
///
/// eg. /transport/connect?type=send
/// eg. /transport/connect?type=recv
///
#[post("/voice/transport/connect")]
pub async fn connect_transport(
    client: VoiceClientEx,
    request: Json<ConnectTransportRequest>,
    query: Query<TransportTypeQuery>,
) -> ConnectTransportResponse {
    use ConnectTransportError::*;
    use TransportType::*;

    let transport_to_connect = match query.transport_type {
        Send => &client.c2s_transport,
        Receive => &client.s2c_transport,
    };

    // read lock here is held across an await? too bad!
    // no but in all seriousness, this is probably not a good idea...
    transport_to_connect
        .read()
        .unwrap()
        .as_ref()
        .ok_or_else(|| {
            warn!(
                "client[{:?}]: tried to connect transport {:?} before creating it",
                client.identity, query.transport_type
            );
            TransportNotCreated
        })?
        .connect(WebRtcTransportRemoteParameters {
            dtls_parameters: request.dtls_parameters.clone(),
        })
        .await
        .map_err(|e| {
            error!(
                "client[{:?}]: {:?} connect failed: {}",
                client.identity, query.transport_type, e
            );
            TransportConnectFailed
        })?;

    info!(
        "client[{:?}]: {:?} transport connected",
        client.identity, query.transport_type
    );

    Ok("connected")
}
