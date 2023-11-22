use utoipa::OpenApi;

pub mod login;
pub mod logout;
pub mod register;
pub mod reissue;
pub mod whoami;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(login::login)
        .service(logout::logout)
        .service(reissue::reissue)
        .service(register::register)
        .service(whoami::whoami);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "identity")
    ),
    paths(
        login::login,
        logout::logout,
        reissue::reissue,
        register::register,
        whoami::whoami
    ),
    components(schemas(
        super::user::User,
        super::token::Token,
        super::access_token::AccessToken,
        login::LoginRequest,
        login::LoginResponse,
        reissue::ReissueRequest,
        reissue::ReissueResponse,
        register::RegisterRequest,
    ))
)]
pub struct AuthApiDocs;
