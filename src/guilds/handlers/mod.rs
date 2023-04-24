use actix_web::web;

pub mod channel;
pub mod guild;

pub fn scope() -> actix_web::Scope {
    web::scope("/guilds")
        .service(guild::create_guild)
        .service(guild::delete_guild)
        .service(guild::list_guilds)
        .service(guild::join_guild)
}
