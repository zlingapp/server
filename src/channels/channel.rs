use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Copy, Clone, sqlx::Type, Serialize, Deserialize, Debug, ToSchema)]
#[sqlx(type_name = "channel_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}
