pub mod connect_transport;
pub mod consume;
pub mod create_transport;
pub mod join_vc;
pub mod leave_vc;
pub mod list_vc_peers;
pub mod produce;
pub mod voice_events;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(list_vc_peers::list_vc_peers)
        .service(join_vc::join_vc)
        .service(leave_vc::leave_vc)
        .service(voice_events::voice_events_ws)
        .service(create_transport::create_transport)
        .service(connect_transport::connect_transport)
        .service(produce::handle_produce)
        .service(consume::handle_consume);
}
