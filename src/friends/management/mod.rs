use utoipa::OpenApi;

pub mod add_friend;
pub mod list_friend_requests;
pub mod list_friends;
pub mod remove_friend;
pub mod remove_friend_request;
use crate::friends::friend_request::{FriendRequest, FriendRequestType};

pub fn configure_app(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(list_friend_requests::list_friend_requests);
    cfg.service(list_friends::list_friends);
    cfg.service(remove_friend::remove_friend);
    cfg.service(add_friend::add_friend);
    cfg.service(remove_friend_request::remove_friend_request);
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "friends")
    ),
    paths(
        list_friends::list_friends,
        list_friend_requests::list_friend_requests,
        remove_friend::remove_friend,
        add_friend::add_friend,
        remove_friend_request::remove_friend_request
    ),
    components(schemas(
        FriendRequest,
        FriendRequestType
    ))
)]
pub struct FriendsManagementApiDoc;
