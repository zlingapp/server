use crate::auth::user::PublicUserInfo;
use actix_web::web::Path;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Copy, Clone, Serialize, Deserialize, Debug, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FriendRequestType {
    Incoming,
    Outgoing,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct FriendRequest {
    pub direction: FriendRequestType,
    pub user: PublicUserInfo,
}

#[derive(Serialize, Deserialize, IntoParams)]
pub struct UserIdParams {
    pub user_id: String,
}

pub type UserIdPath = Path<UserIdParams>;
