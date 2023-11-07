use utoipa::OpenApi;

pub mod consumer_manager;
pub mod consumer_map;
pub mod events;
pub mod topic;

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
