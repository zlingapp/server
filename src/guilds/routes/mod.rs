use actix_web::web::Path;
use serde::Deserialize;

pub mod create_guild;
pub mod delete_guild;
pub mod join_guild;
pub mod list_joined_guilds;
pub mod update_guild;

#[derive(Deserialize)]
pub struct GuildPathInner {
    pub guild_id: String,
}

pub type GuildPath = Path<GuildPathInner>;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create_guild::create_guild)
        .service(delete_guild::delete_guild)
        .service(join_guild::join_guild)
        .service(list_joined_guilds::list_joined_guilds)
        .service(update_guild::update_guild);
}
