use utoipa::OpenApi;

use self::upload::{UploadedFileInfo, UploadedFileType};

pub mod getfile;
pub mod upload;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(getfile::getfile).service(upload::upload);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "media")
    ),
    paths(
        getfile::getfile,
        upload::upload
    ),
    components(schemas(
        UploadedFileInfo,
        UploadedFileType
    ))
)]
pub struct MediaApiDocs;
