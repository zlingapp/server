use std::fs::File;

use actix_files::NamedFile;
use actix_web::{error::ErrorNotFound, get, web::Path, Error, Responder};
use serde::Deserialize;

use crate::{media::FILENAME_REGEX, options};

#[derive(Deserialize)]
pub struct FileIdentifierPath {
    pub id: String,
    pub filename: String,
}

#[get("/media/{id}/{filename}")]
pub async fn getfile(req: Path<FileIdentifierPath>) -> Result<impl Responder, Error> {
    let filename = req.id.clone() + "_" + &req.filename;

    // path traversal prevention
    if !FILENAME_REGEX.is_match(&filename) {
        return Err(ErrorNotFound("not_found"));
    }

    let path = (*options::MEDIA_PATH).to_string() + "/" + &filename;

    let file = File::open(path).map_err(|_| ErrorNotFound("not_found"))?;
    NamedFile::from_file(file, &req.filename).map_err(|_| ErrorNotFound("not_found"))
}
