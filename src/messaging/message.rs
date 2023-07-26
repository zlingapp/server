use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{auth::user::PublicUserInfo, media::routes::upload::UploadedFileInfo};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<UploadedFileInfo>>,
    pub created_at: DateTime<Utc>,
    pub author: MessageAuthor,
}

// TODO: add nicknames
pub type MessageAuthor = PublicUserInfo;

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
