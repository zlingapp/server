use utoipa::{OpenApi, openapi};

pub mod login;
pub mod logout;
pub mod register;
pub mod whoami;
pub mod reissue;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(login::login)
        .service(logout::logout)
        .service(reissue::reissue)
        .service(register::register)
        .service(whoami::whoami);
}

#[derive(OpenApi)]
#[openapi(
    paths(whoami::whoami),
)]
pub(crate) struct SpecialApi;