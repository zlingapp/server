use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{auth::user::PublicUserInfo, media::routes::upload::UploadedFileInfo};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    #[schema(example = "K1vqjuY8OqU0VO7oJlGpY")]
    pub id: String,
    #[schema(example = "Good morning!")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<UploadedFileInfo>>,
    pub created_at: DateTime<Utc>,
    pub author: PublicUserInfo,
}

// TODO: add nicknames
// pub type MessageAuthor = PublicUserInfo;

// #[derive(Serialize, Deserialize)]
// #[serde(tag = "type", rename_all = "camelCase")]
// pub enum Attachment {
//     Image {
//         url: String,
//         #[serde(skip_serializing_if = "Option::is_none")]
//         thumbnail_url: Option<String>,
//         // width: usize,
//         // height: usize
//     },
//     Blob {
//         url: String,
//         filename: String,
//         size: usize
//     }
// }
