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
