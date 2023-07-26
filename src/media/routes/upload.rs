use std::io::{self, ErrorKind};

use actix_multipart::{Field, Multipart};
use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorPayloadTooLarge},
    post,
    web::Json,
    Error, HttpRequest,
};
use futures::TryStreamExt;
use log::warn;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::{
    auth::access_token::AccessToken,
    media::{util::clean_filename, FILENAME_REGEX},
    options,
};

const MAX_FILE_SIZE: usize = 250 * 1_000_000; // 250 MB

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFileInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub r#type: UploadedFileType,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UploadedFileType {
    Blob,
    Image,
    Video,
    Audio,
    Text,
}

#[post("/media/upload")]
pub async fn upload(
    _token: AccessToken,
    mut payload: Multipart,
    request: HttpRequest,
) -> Result<Json<UploadedFileInfo>, Error> {
    let payload_size = request
        .headers()
        .get("content-length")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();

    if payload_size > MAX_FILE_SIZE {
        return Err(ErrorPayloadTooLarge("file_size_exceeds_limit"));
    }

    while let Some(field) = payload.try_next().await.unwrap() {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap();

        if field_name != "file" {
            continue;
        }

        let filename = content_disposition.get_filename().map(String::from);
        let filename = match filename.map(|v| clean_filename(v)).flatten() {
            Some(n) => n,
            None => format!("file-{}", nanoid!(6)),
        };

        if filename.len() > 64 {
            return Err(ErrorBadRequest("filename_too_long"));
        }

        // get the file type
        use UploadedFileType::*;
        let r#type: UploadedFileType = match field.content_type() {
            Some(t) => match t.type_() {
                mime::TEXT => Text,
                mime::IMAGE => Image,
                mime::AUDIO => Audio,
                mime::VIDEO => Video,
                _ => Blob,
            },
            None => Blob,
        };

        let id = nanoid!();
        {
            let filename = id.clone() + "_" + &filename;

            // final check for file name validity, just to be sure...
            if !FILENAME_REGEX.is_match(&filename) {
                warn!("filename invalid after cleaning: `{}`", filename);
                return Err(ErrorBadRequest("invalid_name"));
            }

            let path = (*options::MEDIA_PATH).to_string() + "/" + &filename;

            let result = save_file(&path, field).await;
            if let Err(e) = result {
                warn!("saving usermedia `{}` failed: {}", filename, e);
                return Err(ErrorInternalServerError("could_not_save"));
            }
        }

        let url = format!("/api/media/{}/{}", id, filename);
        return Ok(Json(UploadedFileInfo {
            id,
            name: filename,
            url,
            r#type,
        }));
    }

    return Err(ErrorBadRequest("missing_file_field"));
}

async fn save_file(path: &str, mut field: Field) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;

    // actually save the file
    while let Ok(chunk) = field.try_next().await {
        if chunk.is_none() {
            return Ok(()); // end of stream
        }
        file.write_all(&chunk.unwrap()).await?;
    }

    Err(io::Error::new(ErrorKind::Other, "error_polling_next_chunk"))
}
