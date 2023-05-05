use actix_web::{
    cookie::{Cookie, CookieBuilder, SameSite}
};

pub mod login;
pub mod logout;
pub mod register;
pub mod whoami;

fn build_session_cookie(session: &str) -> CookieBuilder {
    Cookie::build("Session", session)
        // only allow https
        .secure(true)
        // disallow js access
        .http_only(true)
        // send on all api calls
        .path("/api")
        // allow cookie to be sent on anywhere on the same domain, even if the user is coming from another site
        .same_site(SameSite::Lax)
}

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(login::login)
        .service(logout::logout)
        .service(register::register)
        .service(whoami::whoami);
}
