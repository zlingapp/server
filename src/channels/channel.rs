use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "channel_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}