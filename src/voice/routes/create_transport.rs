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
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::options::webrtc_transport_options;
use crate::voice::{client::VoiceClientEx, transport::TransportType};

/*
   There are two handlers registered for transports.
   - POST /transports/create (defined in create_transport)
   - POST /transports/connect (defined in connect_transport)

   The first one creates a new transport, and the second one connects it to the remote peer.
   The same endpoint is used for both send and receive transports, and the type is specified in
   the query string. This is because the process for creating a send and receive transport is the same.
*/

/// This struct is used to deserialize the query string of the request.
/// It contains the type of transport to create.
/// In the URL, it looks like ?type=send or ?type=recv
#[derive(Debug, Deserialize, IntoParams)]
pub struct TransportTypeQuery {
    #[param(style = Form)]
    #[serde(rename = "type")]
    pub transport_type: TransportType,
}

/// This is what the server will reply with when a transport is created. This
/// should be enough for the client to create a counterpart to whatever
/// transport it needs. You may feed this entire object into
/// `device.createSendTransport` or `device.createRecvTransport`.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransportReply {
    #[schema(value_type = uuid::Uuid)]
    id: String,
    #[schema(value_type = Object)]
    ice_parameters: IceParameters,
    #[schema(value_type = Object)]
    ice_candidates: Vec<IceCandidate>,
    #[schema(value_type = Object)]
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

/// Create WebRTC transport
/// 
/// Allocate a server-side client or serverbound mediasoup WebRTC transport. The
/// direction of the transport is specified in the `type` parameter. You may
/// only create one `send` or `recv` transport per voice connection, and any
/// duplicate requests will be met with `409 Conflict`.
/// 
/// A call to create a transport should be followed by a call to connect the
/// transport. To send voice data, you must create and connect a `send`
/// transport. Conversely, to receive voice data, you must create and connect a
/// `recv` transport.
#[utoipa::path(
    tag = "voice",
    security(("voice" = [])),
    params(TransportTypeQuery),
    responses(
        (status = OK, description = "Ready for you to start consuming", body = CreateTransportReply),
        (status = CONFLICT, description = "Transport of this type already created"),
        (status = INTERNAL_SERVER_ERROR, description = "Transport allocation failed on the serverside"),
    )
)]
#[post("/voice/transport/create")]
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
