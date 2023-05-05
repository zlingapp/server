pub mod create_channel;
pub mod delete_channel;
pub mod list_guild_channels;
pub mod update_channel;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create_channel::create_channel)
        .service(list_guild_channels::list_guild_channels);
}
