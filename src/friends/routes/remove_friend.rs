use crate::{
    auth::access_token::AccessToken,
    db::DB,
    error::macros::err,
    error::HResult,
    friends::friend_request::{UserIdParams, UserIdPath},
    realtime::pubsub::pubsub::{Event, PubSub},
};
use actix_web::{
    delete,
    web::{Data, Json},
};

/// Remove a friend
///
/// Removes a user from your friend list
#[utoipa::path(
    params(UserIdParams),
    responses(
        (status = OK, description="Friends list", body=Vec<PublicUserInfo>),
        (status = BAD_REQUEST, description="You are not friends with that user", example="not_friends")
    ),
    tag="friends",
    security(("token" = []))
)]
#[delete("/friends/{user_id}")]
pub async fn remove_friend(
    db: DB,
    pubsub: Data<PubSub>,
    path: UserIdPath,
    token: AccessToken,
) -> HResult<Json<String>> {
    if !db.is_user_friend(&token.user_id, &path.user_id).await? {
        return err!(400, "You are not friends with that user");
    }
    sqlx::query!(
        r#"UPDATE users
            SET friends = ARRAY_REMOVE(friends,$1)
            WHERE id=$2"#,
        path.user_id,
        token.user_id
    )
    .execute(&db.pool)
    .await?;

    // Let them know that we no longer require their services
    let me_user = db
        .get_user_by_id(&token.user_id)
        .await?
        .expect("A user not in the db sent an authenticated request?? WTF!!!");
    pubsub.send_to(
        &path.user_id,
        Event::FriendRemove {
            user: &me_user.into(),
        },
    )
    .await;

    Ok(Json("success".to_string()))
}
