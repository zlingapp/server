use std::fs::File;

use actix_files::NamedFile;
use actix_web::{error::ErrorNotFound, get, web::Path, Error};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::{media::FILENAME_REGEX, options};

#[derive(Deserialize, IntoParams)]
pub struct FileIdentifierPath {
    #[param(example = "s6NIiu2oOh1FEL0Xfjc7n")]
    pub id: String,
    #[param(example = "cat.jpg")]
    pub filename: String,
}

// just a helper struct to describe what a file looks like to openapi
#[derive(ToSchema)]
#[schema(example = "(binary file body)")]
struct OApiFileResponse {}

/// Download file
/// 
/// Retreives the requested file's contents bit-for-bit as it was when uploaded.
/// This endpoint does not currently require authentication, which means that
/// external references to images hosted on Zling can be made.
#[utoipa::path(
    tag = "media",
    params(FileIdentifierPath),
    responses(
        (status = OK, description = "Requested file", content_type = "multipart", body = inline(OApiFileResponse)),
        (status = NOT_FOUND, description = "File not found", example = "not_found")
    )
)]
#[get("/media/{id}/{filename}")]
pub async fn getfile(req: Path<FileIdentifierPath>) -> Result<NamedFile, Error> {
    let filename = req.id.clone() + "_" + &req.filename;

    // path traversal prevention
    if !FILENAME_REGEX.is_match(&filename) {
        return Err(ErrorNotFound("not_found"));
    }

    let path = (*options::MEDIA_PATH).to_string() + "/" + &filename;

    let file = File::open(path).map_err(|_| ErrorNotFound("not_found"))?;
    NamedFile::from_file(file, &req.filename).map_err(|_| ErrorNotFound("not_found"))
}
