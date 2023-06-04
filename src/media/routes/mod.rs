pub mod getfile;
pub mod upload;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(getfile::getfile)
        .service(upload::upload);
}
