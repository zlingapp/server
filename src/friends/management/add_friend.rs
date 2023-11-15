use actix_web::{
    post,
    web::{Data, Json},
};
use sqlx::query;
use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    friends::friend_request::{UserIdParams, UserIdPath},
    realtime::pubsub::pubsub::{Event, PubSub},
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
    if db.is_user_friend(&me.id, &path.user_id).await? {
        err!(400, "You are already friends with that user")?;
    }
    let incoming = db.list_incoming_friend_requests(&me.id).await?;
    if incoming.iter().any(|i| i.user.id == path.user_id) {
        // We have an incoming friend request, add friends now
        db.add_friends(&path.user_id, &me.id).await?;

        query!(
            r#"DELETE FROM friend_requests
            WHERE from_user=$1
            AND to_user=$2"#,
            &path.user_id,
            &me.id
        )
        .execute(&db.pool)
        .await?;

        // Notify the other party that their request has been accepted

        pubsub
            .send_to(
                &path.user_id,
                Event::FriendRequestUpdate { user: &me.into() },
            )
            .await;

        return Ok(Json("Friend successfully added".into()));
    }
    let outgoing = db.list_outgoing_friend_requests(&me.id).await?;
    if outgoing.iter().any(|i| i.user.id == path.user_id) {
        err!(
            400,
            "An outgoing friend request to this user already exists"
        )?;
    }

    // Now we should create an outgoing friend request
    sqlx::query!(
        r#"INSERT INTO friend_requests
            VALUES ($1,$2)"#,
        &me.id,
        &path.user_id
    )
    .execute(&db.pool)
    .await?;

    // Notify the other party that I am now your friend
    pubsub
        .send_to(
            &path.user_id,
            Event::FriendRequestUpdate { user: &me.into() },
        )
        .await;

    Ok(Json("Friend request sucessfully sent".into()))
}
