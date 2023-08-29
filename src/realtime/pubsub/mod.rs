use utoipa::OpenApi;

pub mod events;
pub mod topic;
pub mod consumer_map;
pub mod consumer_manager;
pub mod consumer;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "pubsub")
    ),
    paths(
        events::events_ws
    ),
)]
pub struct PubSubApiDoc;