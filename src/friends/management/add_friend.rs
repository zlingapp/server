use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    friends::friend_request::{UserIdParams, UserIdPath},
    realtime::pubsub::pubsub::PubSub,
};
use actix_web::{
    post,
    web::{Data, Json},
};

/// Add a friend
///
/// If the user has an incoming friend request from this user, it will accept
/// the request and add friend. Otherwise, it will send them a friend request.
#[utoipa::path(
    params(UserIdParams),
    responses(
        (status = OK, description = "Friend request accepted", body = String),
        (status = OK, description = "Friend request sent", body = String),
        (status = BAD_REQUEST, description = "You are already friends with that user", body=String),
        (status = BAD_REQUEST, description = "An outgoing friend request to that user already exists", body=String)
    ),
    tag = "friends",
    security(("token" = []))
)]
#[post("/friends/requests/{user_id}")]
pub async fn add_friend(
    db: DB,
    pubsub: Data<PubSub>,
    me: User,
    path: UserIdPath,
) -> HResult<Json<String>> {
    if me.id == path.user_id {
        err!(400, "You cannot add yourself as a friend")?;
    }

    if db.is_user_friend(&me.id, &path.user_id).await? {
        err!(400, "You are already friends with that user")?;
    }

    let incoming_deleted = sqlx::query!(
        r#"DELETE FROM friend_requests WHERE to_user=$1 AND from_user=$2"#,
        &me.id,
        &path.user_id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if incoming_deleted > 0 {
        db.add_friends(&path.user_id, &me.id).await?;

        // Notify the other party that their request has been accepted
        pubsub
            .notify_friend_request_accepted(&path.user_id, &me.into())
            .await;

        // TODO: standardize responses
        return Ok(Json("Friend added".into()));
    }

    // there is no incoming friend request, so we should create an outgoing one
    let rows_affected = sqlx::query!(
        r#"INSERT INTO friend_requests (from_user, to_user) VALUES ($1, $2) ON CONFLICT DO NOTHING"#,
        &me.id,
        &path.user_id
    )
    .execute(&db.pool)
    .await?.rows_affected();

    if rows_affected == 0 {
        err!(
            400,
            "An outgoing friend request to that user already exists"
        )?;
    }

    // Notify the other party that we sent them a friend request
    pubsub
        .notify_friend_request_sent(&path.user_id, &me.into())
        .await;

    Ok(Json("Friend request sent".into()))
}
