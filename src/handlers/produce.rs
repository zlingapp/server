// implement the produce request
use actix_web::error::ResponseError;
use actix_web::post;
use actix_web::web::Json;
use derive_more::{Display, Error};
use log::error;
use mediasoup::rtp_observer::{RtpObserver, RtpObserverAddProducerOptions};
use mediasoup::{
    producer::ProducerOptions,
    rtp_parameters::{MediaKind, RtpParameters},
    transport::Transport,
};
use serde::{Deserialize, Serialize};

use crate::client::ClientEx;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProduceRequest {
    pub kind: MediaKind,
    pub rtp_parameters: RtpParameters,
}

#[derive(Debug, Display, Error)]
#[display(rename_all = "snake_case")]
pub enum ProduceError {
    TransportNotCreated,
    TransportNotConnected,
    ProducerFailed,
}

impl ResponseError for ProduceError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        use ProduceError::*;
        match self {
            TransportNotCreated => StatusCode::BAD_REQUEST,
            TransportNotConnected => StatusCode::BAD_REQUEST,
            ProducerFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProduceReply {
    pub id: String,
}

pub type ProduceResponse = Result<Json<ProduceReply>, ProduceError>;

#[post("/produce")]
pub async fn handle_produce(client: ClientEx, request: Json<ProduceRequest>) -> ProduceResponse {
    if client.c2s_transport.read().unwrap().is_none() {
        return Err(ProduceError::TransportNotCreated);
    }

    let producer;

    {
        let transport = client.c2s_transport.read().unwrap();
        let transport = transport.as_ref().unwrap(); // this is a safe unwrap because of the check above

        if transport.closed() {
            return Err(ProduceError::TransportNotConnected);
        }

        producer = transport
            .produce(ProducerOptions::new(
                request.kind,
                request.rtp_parameters.clone(),
            ))
            .await
            .map_err(|e| {
                error!("client[{:?}]: produce() failed: {}", client.identity, e);
                ProduceError::ProducerFailed
            })?
    };

    client
        .channel
        .al_observer
        .add_producer(RtpObserverAddProducerOptions::new(producer.id()))
        .await
        .unwrap();

    let id = producer.id().to_string();
    let kind = producer.kind();

    client
        .producers
        .lock()
        .unwrap()
        .insert(id.clone(), producer);

    client.channel.notify_new_producer(&client, id.clone(), kind).await;

    Ok(Json(ProduceReply { id }))
}
