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
use serde_json::{json, Value};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::{
    auth::access_token::AccessToken,
    media::{util::clean_filename, FILENAME_REGEX},
    options,
};

const MAX_FILE_SIZE: usize = 250 * 1_000_000; // 250 MB

#[post("/media/upload")]
pub async fn upload(
    _token: AccessToken,
    mut payload: Multipart,
    request: HttpRequest,
) -> Result<Json<Value>, Error> {
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

        let file_name = content_disposition.get_filename().map(String::from);

        let filename = match file_name.map(|v| clean_filename(v)).flatten() {
            Some(n) => n,
            None => "unknown-".to_owned() + &nanoid!(5),
        };

        if filename.len() > 64 {
            return Err(ErrorBadRequest("filename_too_long"));
        }

        let id = nanoid!();
        {
            let filename = id.clone() + "_" + &filename;

            // final check for file name validity, just to be sure...
            if !FILENAME_REGEX.is_match(&filename) {
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
        return Ok(Json(json!({
            "id": id,
            "name": filename,
            "url": url
        })));
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
