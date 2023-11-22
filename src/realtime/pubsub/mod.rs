use utoipa::OpenApi;

pub mod events;
#[allow(clippy::module_inception)] // This should really have a different name...
pub mod pubsub;
pub mod pubsub_map;
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
