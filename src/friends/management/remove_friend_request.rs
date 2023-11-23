use actix_web::{
    delete,
    web::{Data, Json},
};

use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    friends::friend_request::{UserIdParams, UserIdPath},
    realtime::pubsub::pubsub::PubSub,
};

/// Remove a friend request
///
/// If there is an incoming friend request from the user, it will deny it
/// If there is an outgoing friend request to the user, it will cancel it
#[utoipa::path(
    params(UserIdParams),
    responses(
        (status = OK, description = "Incoming friend request denied", body = String),
        (status = OK, description = "Outgoing friend request cancelled", body = String),
        (status = BAD_REQUEST, description = "No outgoing or incoming friend request with that user", body = String)
    ),
    tag = "friends",
    security(("token" = []))
)]
#[delete("/friends/requests/{user_id}")]
pub async fn remove_friend_request(
    db: DB,
    pubsub: Data<PubSub>,
    me: User,
    path: UserIdPath,
) -> HResult<Json<String>> {
    if me.id == path.user_id {
        err!(400, "You cannot remove a friend request to yourself")?;
    }

    let incoming_deleted = sqlx::query!(
        r#"DELETE FROM friend_requests
            WHERE to_user=$1
            AND from_user=$2"#,
        &me.id,
        &path.user_id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if incoming_deleted > 0 {
        // Notify them that we hate their guts and denied their friend request
        pubsub
            .notify_friend_request_remove(&path.user_id, &me.into())
            .await;

        return Ok(Json("Incoming friend request successfully denied".into()));
    }

    let outgoing_deleted = sqlx::query!(
        r#"DELETE FROM friend_requests
            WHERE to_user=$1
            AND from_user=$2"#,
        &path.user_id,
        &me.id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if outgoing_deleted > 0 {
        pubsub
            .notify_friend_request_remove(&path.user_id, &me.into())
            .await;
        return Ok(Json("Outgoing friend request sucessfully cancelled".into()));
    }

    err!(400, "No incoming or outgoing friend request with that user")
}
