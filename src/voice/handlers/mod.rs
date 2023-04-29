pub mod join_leave;
use actix_web::web;

pub use self::join_leave::join_vc;
pub use self::join_leave::leave_vc;

pub mod transports;
pub use self::transports::{connect_transport, create_transport};

pub mod produce;
pub use self::produce::handle_produce;

pub mod realtime;
pub use self::realtime::voice_events_ws;

pub mod query;
pub use self::query::query_channel;

pub mod consume;
pub use self::consume::handle_consume;

pub fn scope() -> actix_web::Scope {
    web::scope("/voice")
        .service(query_channel)
        .service(join_vc)
        .service(leave_vc)
        .service(voice_events_ws)
        .service(create_transport)
        .service(connect_transport)
        .service(handle_produce)
        .service(handle_consume)
}
