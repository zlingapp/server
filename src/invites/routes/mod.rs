pub mod create_invite;
pub mod delete_invite;
pub mod peek_invite;
pub mod use_invite;
pub mod list_invites;

use list_invites::InviteInfo;
use create_invite::{CreateInviteRequest, CreateInviteResponse};
use utoipa::OpenApi;

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create_invite::create_invite);
    cfg.service(peek_invite::peek_invite);
    cfg.service(use_invite::use_invite);
    cfg.service(delete_invite::delete_invite);
    cfg.service(list_invites::list_invites);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "invites")
    ),
    paths(
        create_invite::create_invite,
        peek_invite::peek_invite,
        use_invite::use_invite,
        delete_invite::delete_invite,
        list_invites::list_invites,
    ),
    components(schemas(
        CreateInviteRequest,
        CreateInviteResponse,
        InviteInfo
    ))
)]
pub struct InvitesApiDoc;
