use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    #[schema(example = "7UU0KB41")]
    pub code: String,
    pub guild_id: String,
    pub inviter: String,
    #[schema(example = 5)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}
