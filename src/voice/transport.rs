use serde::Deserialize;
use utoipa::ToSchema;

/// This enum is used to specify the type of transport to create.
/// It is used in the query string of the request.
#[derive(Debug, Deserialize, ToSchema)]
pub enum TransportType {
    #[serde(rename = "send")]
    Send,
    #[serde(rename = "recv")]
    Receive,
}
