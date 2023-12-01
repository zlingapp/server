use crate::{
    auth::access_token::AccessToken, db::DB, error::HResult, friends::friend_request::FriendRequest,
};
use actix_web::{get, web::Json};
use utoipa;

/// List Friend Requests
///
/// Lists all incoming and outgoing friend requests.
#[utoipa::path(
    responses(
        (status = OK, description="Friend requests list", body=Vec<FriendRequest>),
    ),
    tag = "friends",
    security(("token" = []))
)]
#[get("/friends/requests")]
pub async fn list_friend_requests(db: DB, token: AccessToken) -> HResult<Json<Vec<FriendRequest>>> {
    let mut incoming = db.list_incoming_friend_requests(&token.user_id).await?;
    let outgoing = db.list_outgoing_friend_requests(&token.user_id).await?;
    incoming.extend(outgoing);
    Ok(Json(incoming))
}
