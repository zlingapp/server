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
use utoipa::{ToSchema};

use crate::{
    auth::access_token::AccessToken,
    media::{util::clean_filename, FILENAME_REGEX},
    options,
};

const MAX_FILE_SIZE: usize = 250 * 1_000_000; // 250 MB

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFileInfo {
    #[schema(example = "s6NIiu2oOh1FEL0Xfjc7n")]
    pub id: String,
    #[schema(example = "cat.jpg")]
    pub name: String,
    #[schema(example = "/api/media/s6NIiu2oOh1FEL0Xfjc7n/cat.jpg")]
    pub url: String,
    pub r#type: UploadedFileType,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = "image")]
pub enum UploadedFileType {
    Blob,
    Image,
    Video,
    Audio,
    Text,
}

// this struct isn't actually used in useful code, just the openapi definition
#[derive(ToSchema)]
pub struct UploadFormRequest {
    #[allow(dead_code)]
    file: [u8]
}

/// Upload file
/// 
/// Use this endpoint to upload attachments to be hosted on the Zling media
/// server. If your file name is not ASCII alphanumeric, it will be cleaned
/// first before it is saved. You must not exceed the Zling server's filesize
/// limit, which is usually around 250MB.
#[utoipa::path(
    tag = "media",
    security(("token" = [])),
    request_body(
        description = "The file to upload",
        content_type = "multipart/form-data",
        content = inline(UploadFormRequest)
    ),
    responses(
        (status = OK, description = "File uploaded", body = UploadedFileInfo),
        (status = BAD_REQUEST, description = "Invalid file supplied (eg. name could not be cleaned)"),
        (status = PAYLOAD_TOO_LARGE, description = "The file exceeds the server's size limit")
    )    
)]
#[post("/media/upload")]
pub async fn upload(
    _token: AccessToken,
    mut payload: Multipart,
    request: HttpRequest,
) -> Result<Json<UploadedFileInfo>, Error> {
    let payload_size = request
        .headers()
        .get("content-length")
        .ok_or(ErrorBadRequest("invalid_content_length"))?
        .to_str()
        .map_err(|_| ErrorBadRequest("invalid_content_length"))?
        .parse::<usize>()
        .map_err(|_| ErrorBadRequest("invalid_content_length"))?;

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
        // clean the file name
        let filename = match filename.map(|v| clean_filename(v)).flatten() {
            Some(n) => n,
            None => random_file_name(),
        };

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

pub fn random_file_name() -> String {
    format!("file-{}", nanoid!(6))
}