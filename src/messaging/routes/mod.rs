use utoipa::OpenApi;

use crate::auth::user::PublicUserInfo;

use self::send_message::{SendMessageRequest, SendMessageResponse};

use super::message::Message;

pub mod delete_message;
pub mod edit_message;
pub mod read_message_history;
pub mod send_message;
pub mod typing;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(read_message_history::read_message_history)
        .service(send_message::send_message)
        .service(delete_message::delete_message)
        .service(typing::typing);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "messaging")
    ),
    paths(
        read_message_history::read_message_history,
        send_message::send_message,
        delete_message::delete_message,
        typing::typing
    ),
    components(schemas(
        Message, SendMessageResponse, SendMessageRequest, PublicUserInfo
    ))
)]
pub struct MessagingApiDocs;
