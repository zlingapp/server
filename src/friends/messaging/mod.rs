pub mod delete_message;
pub mod read_message_history;
pub mod send_message;
pub mod typing;
use utoipa::OpenApi;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(send_message::send_message);
    cfg.service(delete_message::delete_message);
    cfg.service(typing::typing);
    cfg.service(read_message_history::read_message_history);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "DMs"),
    ),
    paths(
        send_message::send_message,
        delete_message::delete_message,
        typing::typing,
        read_message_history::read_message_history
    ),
    components(
        schemas(
            send_message::SendDMRequest,
            send_message::SendDMResponse,
        )
    )
)]
pub struct FriendsMessagingApiDoc;
