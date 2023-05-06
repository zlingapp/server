pub mod login;
pub mod logout;
pub mod register;
pub mod whoami;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(login::login)
        .service(logout::logout)
        .service(register::register)
        .service(whoami::whoami);
}
