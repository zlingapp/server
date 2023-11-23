pub mod delete_message;
pub mod send_message;
use utoipa::OpenApi;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(send_message::send_message);
    cfg.service(delete_message::delete_message);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "DMs"),
    ),
    paths(
        send_message::send_message,
        delete_message::delete_message,
    ),
    components(
        schemas(
            send_message::SendDMRequest,
            send_message::SendDMResponse,
        )
    )
)]
pub struct FriendsMessagingApiDoc;
