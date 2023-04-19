use actix_web::{
    post,
    web::{Json, Query},
    ResponseError,
};
use derive_more::{Display, Error};
use log::{error, info, warn};
use mediasoup::{
    prelude::{DtlsParameters, IceCandidate, IceParameters},
    transport::Transport,
    webrtc_transport::WebRtcTransportRemoteParameters,
};
use serde::{Deserialize, Serialize};

use crate::voice::{client::VoiceClientEx, options::webrtc_transport_options};

/*
   There are two handlers registered below for transports.
   - POST /transports/create (defined in create_transport)
   - POST /transports/connect (defined in connect_transport)

   The first one creates a new transport, and the second one connects it to the remote peer.
   The same endpoint is used for both send and receive transports, and the type is specified in
   the query string. This is because the process for creating a send and receive transport is the same.
*/

/// This enum is used to specify the type of transport to create.
/// It is used in the query string of the request.
#[derive(Debug, Deserialize)]
pub enum TransportType {
    #[serde(rename = "send")]
    Send,
    #[serde(rename = "recv")]
    Receive,
}
/// This struct is used to deserialize the query string of the request.
/// It contains the type of transport to create.
/// In the URL, it looks like ?type=send or ?type=recv
#[derive(Debug, Deserialize)]
pub struct TransportTypeQuery {
    #[serde(rename = "type")]
    transport_type: TransportType,
}

/// This is what the server will reply with when a transport is created.
/// This should be enough for the client to create a counterpart to whatever transport it needs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransportReply {
    id: String,
    ice_parameters: IceParameters,
    ice_candidates: Vec<IceCandidate>,
    dtls_parameters: DtlsParameters,
}

/// If things go wrong when creating a transport, this enum will be used to specify the error.
#[derive(Debug, Display, Error)]
#[display(rename_all = "snake_case")]
pub enum CreateTransportError {
    TransportAlreadyExists,
    TransportCreateFailed,
}
impl ResponseError for CreateTransportError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        use CreateTransportError::*;
        match self {
            TransportAlreadyExists => StatusCode::CONFLICT,
            TransportCreateFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type CreateTransportResponse = Result<Json<CreateTransportReply>, CreateTransportError>;

/// POST /transport/create
///
///     Creates a new transport. The type of transport is specified in the query string.
///
/// eg. /transport/create?type=send
/// eg. /transport/create?type=recv
#[post("/transport/create")]
pub async fn create_transport(
    client: VoiceClientEx,
    query: Query<TransportTypeQuery>,
) -> CreateTransportResponse {
    use CreateTransportError::*;
    use TransportType::*;

    // Get the transport to assign to. This is either the send or receive transport.
    let transport_to_assign = match query.transport_type {
        Send => &client.c2s_transport,
        Receive => &client.s2c_transport,
    };

    // If the transport already exists, return an error.
    if transport_to_assign.read().unwrap().is_some() {
        warn!(
            "client[{:?}]: tried to create {:?} transport when it already exists",
            client.identity, query.transport_type
        );
        return Err(TransportAlreadyExists);
    }

    // Create the transport.
    let transport = client
        .channel
        .router
        .create_webrtc_transport(webrtc_transport_options())
        .await
        .map_err(|e| {
            error!(
                "client[{:?}]: transport {:?} create failed: {}",
                client.identity, query.transport_type, e
            );
            TransportCreateFailed
        })?;

    // Prepare the reply with all the needed information.
    let reply = CreateTransportReply {
        id: transport.id().to_string(),
        ice_parameters: transport.ice_parameters().clone(),
        ice_candidates: transport.ice_candidates().clone(),
        dtls_parameters: transport.dtls_parameters(),
    };

    info!(
        "client[{:?}]: {:?} transport created, id: {}",
        client.identity,
        query.transport_type,
        transport.id()
    );

    // Write through the RwLock and assign the transport to the client.
    *transport_to_assign.write().unwrap() = Some(transport);

    Ok(Json(reply))
}

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
#[post("/transport/connect")]
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
                "client[{:?}]: tried to connect transport {:?} creating it",
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
