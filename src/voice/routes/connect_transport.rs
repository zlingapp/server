use actix_web::{
    post,
    web::{Json, Query},
    ResponseError,
};
use derive_more::{Display, Error};
use log::{error, info, warn};
use mediasoup::{prelude::DtlsParameters, webrtc_transport::WebRtcTransportRemoteParameters};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::voice::{
    client::VoiceClientEx, routes::create_transport::TransportTypeQuery, transport::TransportType,
};

/// This contains the DTLS parameters of the remote peer, which are needed to connect the transport.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectTransportRequest {
    /// See: [DtlsParameters](https://mediasoup.org/documentation/v3/mediasoup/api/#WebRtcTransportDtlsParameters)
    #[schema(value_type = Object)]
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

/// Connect WebRTC transport
///
/// Connects a WebRTC transport to the remote peer. The direction of the
/// transport is specified in the `type` parameter. Please call this after
/// creating a transport on both the client and server side and you are ready to
/// send/receive data.
#[utoipa::path(
    tag = "voice",
    security(("voice" = [])),
    params(TransportTypeQuery),
    responses(
        (status = OK, description = "Transport connected", example = "connected"),
        (status = BAD_REQUEST, description = "A transport of this type has not been created for this voice connection yet, so it cannot be connected."),
        (status = INTERNAL_SERVER_ERROR, description = "There was an error while establishing the RTP connection for this transport.")
    )
)]
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

    transport_to_connect
        .read()
        .await
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
