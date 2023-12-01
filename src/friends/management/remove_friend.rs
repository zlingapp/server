use crate::{
    auth::access_token::AccessToken,
    db::DB,
    error::macros::err,
    error::HResult,
    friends::friend_request::{UserIdParams, UserIdPath},
    realtime::pubsub::pubsub::PubSub,
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
    if path.user_id == token.user_id {
        return err!(400, "You cannot remove yourself as a friend");
    }

    if !db.is_user_friend(&token.user_id, &path.user_id).await? {
        return err!(400, "You are not friends with that user");
    }

    let mut tx = db.pool.begin().await?;

    sqlx::query!(
        r#"UPDATE users SET friends = ARRAY_REMOVE(friends, $1) WHERE id = $2"#,
        path.user_id,
        token.user_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        r#"UPDATE users SET friends = ARRAY_REMOVE(friends, $1) WHERE id = $2"#,
        token.user_id,
        path.user_id,
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    // Let them know that we no longer require their services
    // note: this unwrap panics if authenticated user does not exist in db but this will rarely happen
    let me_user = db.get_user_by_id(&token.user_id).await?.unwrap();

    pubsub
        .notify_friend_remove(&path.user_id, &me_user.into())
        .await;

    Ok(Json("Friend removed".to_string()))
}
