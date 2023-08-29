use actix_web::error::ResponseError;
use actix_web::post;
use actix_web::web::Json;
use derive_more::{Display, Error};
use log::error;
use mediasoup::prelude::ConsumerOptions;
use mediasoup::rtp_parameters::{RtpCapabilities, RtpParameters, MediaKind};
use mediasoup::{producer::ProducerId, transport::Transport};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::voice::client::VoiceClientEx;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsumeRequest {
    /// The ID of the producer to consume. This is usually the producer of another peer.
    #[schema(value_type = Uuid)]
    pub producer_id: ProducerId,
    /// Your mediasoup `Device`'s `rtpCapabilities` property.
    #[schema(value_type = Object)]
    pub rtp_capabilities: RtpCapabilities,
}

#[derive(Debug, Display, Error)]
#[display(rename_all = "snake_case")]
pub enum ConsumeError {
    TransportNotCreated,
    TransportNotConnected,
    CannotConsume,
    ConsumerFailed,
}

impl ResponseError for ConsumeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        use ConsumeError::*;
        match self {
            TransportNotCreated => StatusCode::BAD_REQUEST,
            TransportNotConnected => StatusCode::BAD_REQUEST,
            CannotConsume => StatusCode::BAD_REQUEST,
            ConsumerFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// You can feed this whole JSON object back into the `consume` method of a recv
/// `Transport`.
/// ([Example](https://github.com/zlingapp/zvelte/blob/903fbe7e5c08dbc1279753aa260790e0bf5c23c8/src/components/voice/VoiceManager.svelte#L532))
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsumeReply {
    #[schema(value_type = uuid::Uuid)]
    pub id: String,
    #[schema(value_type = uuid::Uuid)]
    pub producer_id: ProducerId,
    #[schema(value_type = String, example = "audio")]
    pub kind: MediaKind,
    #[schema(value_type = Object)]
    pub rtp_parameters: RtpParameters,
}

pub type ConsumeResponse = Result<Json<ConsumeReply>, ConsumeError>;

/// Consume
/// 
/// Satisfies a clientside `recv` transport's `consume` event by creating a
/// server-side consumer for a certain server-side producer. In other words, it
/// allows you to receive the data of a producer, which is usually owned by
/// another peer, which lets you receive the peer's audio/video.
/// 
/// This endpoint requires a created and connected `recv` transport.
#[utoipa::path(
    tag = "voice",
    security(("voice" = [])),
    responses(
        (status = OK, description = "Server-side consumer created and bound", body = ConsumeReply),
        (status = BAD_REQUEST, description = "Transport not created & connected yet, or media codec unsupported by consumer"),
    )
)]
#[post("/voice/consume")]
pub async fn handle_consume(client: VoiceClientEx, request: Json<ConsumeRequest>) -> ConsumeResponse {
    use ConsumeError::*;

    if client.s2c_transport.read().unwrap().is_none() {
        return Err(TransportNotCreated);
    }

    if !client
        .channel
        .router
        .can_consume(&request.producer_id, &request.rtp_capabilities)
    {
        return Err(CannotConsume);
    }

    let consumer;

    {
        let transport = client.s2c_transport.read().unwrap();
        let transport = transport.as_ref().unwrap();

        if transport.closed() {
            return Err(TransportNotConnected);
        }

        consumer = transport
            .consume(ConsumerOptions::new(
                request.producer_id,
                request.rtp_capabilities.clone(),
            ))
            .await
            .map_err(|e| {
                error!(
                    "client[{:?}]: consume() on producer {} failed: {}",
                    client.identity, request.producer_id, e
                );
                ConsumerFailed
            })?;
    }

    let id = consumer.id().to_string();

    let reply = ConsumeReply {
        id: id.clone(),
        producer_id: request.producer_id,
        kind: consumer.kind(),
        rtp_parameters: consumer.rtp_parameters().clone(),
    };

    client.consumers.lock().unwrap().insert(id, consumer);

    Ok(Json(reply))
}
