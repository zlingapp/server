pub mod create_bot;
pub mod list_bots;
// pub mod update_bot;
pub mod delete_bot;
pub mod token_reset;

use utoipa::OpenApi;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create_bot::create_bot)
        .service(list_bots::list_bots)
        .service(delete_bot::delete_bot)
        .service(token_reset::token_reset);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "bots", description = "Bot related operations")
    ),
    paths(
        create_bot::create_bot,
        list_bots::list_bots,
        delete_bot::delete_bot,
        token_reset::token_reset
    ),
    components(schemas(
        create_bot::CreateBotRequest,
        create_bot::BotDetails,
        token_reset::TokenResetResponse,
    ))
)]
pub struct BotsApiDoc;
