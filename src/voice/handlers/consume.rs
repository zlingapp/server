use actix_web::error::ResponseError;
use actix_web::post;
use actix_web::web::Json;
use derive_more::{Display, Error};
use log::error;
use mediasoup::prelude::ConsumerOptions;
use mediasoup::rtp_parameters::{RtpCapabilities, RtpParameters, MediaKind};
use mediasoup::{producer::ProducerId, transport::Transport};
use serde::{Deserialize, Serialize};

use crate::voice::client::VoiceClientEx;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumeRequest {
    pub producer_id: ProducerId,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumeReply {
    pub id: String,
    pub producer_id: ProducerId,
    pub kind: MediaKind,
    pub rtp_parameters: RtpParameters,
}

pub type ConsumeResponse = Result<Json<ConsumeReply>, ConsumeError>;

#[post("/consume")]
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
