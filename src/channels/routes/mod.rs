use utoipa::OpenApi;

pub mod create_channel;
pub mod delete_channel;
pub mod list_guild_channels;
pub mod update_channel;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create_channel::create_channel)
        .service(list_guild_channels::list_guild_channels);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "channels")
    ),
    paths(
        create_channel::create_channel,
        list_guild_channels::list_guild_channels
    ),
    components(schemas(
        create_channel::CreateChannelRequest,
        create_channel::CreateChannelResponse,
        list_guild_channels::ChannelInfo,
        super::channel::ChannelType
    ))
)]
pub struct ChannelsApiDocs;
