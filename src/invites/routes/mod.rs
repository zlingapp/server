pub mod create_invite;
pub mod see_invite;
pub mod use_invite;

use create_invite::{CreateInviteRequest, CreateInviteResponse};
use utoipa::OpenApi;
use crate::guilds::routes::list_joined_guilds::GuildInfo;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create_invite::create_invite);
    cfg.service(see_invite::see_invite);
    cfg.service(use_invite::use_invite);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "invites")
    ),
    paths(
        create_invite::create_invite,
        see_invite::see_invite,
        use_invite::use_invite
    ),
    components(schemas(
        CreateInviteRequest,
        CreateInviteResponse,
        GuildInfo
    ))
)]
pub struct InvitesApiDoc;
