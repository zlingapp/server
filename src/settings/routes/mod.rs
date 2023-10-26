pub mod avatar;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(avatar::set_avatar);
}
