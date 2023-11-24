use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    realtime::pubsub::pubsub::PubSub,
};
use actix_web::{
    post,
    web::{Data, Json},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
pub enum AddFriendRequest {
    ById { id: String },
    ByName { username: String },
}

/// Add a friend
///
/// If the user has an incoming friend request from this user, it will accept
/// the request and add friend. Otherwise, it will send them a friend request.
///
/// This endpoint supports two ways of adding a friend:
/// - By ID: `POST /friends/requests { "id": "..." }`
/// - By username: `POST /friends/requests { "username": "..." }`
#[utoipa::path(
    responses(
        (status = OK, description = "Friend request accepted", body = String),
        (status = OK, description = "Friend request sent", body = String),
        (status = NOT_FOUND, description = "User does not exist", body=String),
        (status = BAD_REQUEST, description = "You are already friends with that user", body=String),
        (status = BAD_REQUEST, description = "An outgoing friend request to that user already exists", body=String)
    ),
    tag = "friends",
    security(("token" = []))
)]
#[post("/friends/requests")]
pub async fn add_friend(
    db: DB,
    pubsub: Data<PubSub>,
    me: User,
    req: Json<AddFriendRequest>,
) -> HResult<Json<String>> {
    let user = match req.into_inner() {
        AddFriendRequest::ById { id } => {
            if me.id == id {
                err!(400, "You cannot add yourself as a friend")?;
            }

            let user = db.get_user_by_id(&id).await?;

            match user {
                Some(user) => user,
                None => err!(404, "User not found")?,
            }
        }
        AddFriendRequest::ByName { username } => {
            let user = db.get_user_by_username(&username, true).await?;

            match user {
                Some(user) => {
                    if me.id == user.id {
                        err!(400, "You cannot add yourself as a friend")?;
                    }

                    user
                }
                None => err!(404, "User not found")?,
            }
        }
    };

    if db.is_user_friend(&me.id, &user.id).await? {
        err!(400, "You are already friends with that user")?;
    }

    let incoming_deleted = sqlx::query!(
        r#"DELETE FROM friend_requests WHERE to_user=$1 AND from_user=$2"#,
        &me.id,
        &user.id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if incoming_deleted > 0 {
        db.add_friends(&user.id, &me.id).await?;

        // Notify the other party that their request has been accepted
        pubsub
            .notify_friend_request_accepted(&user.id, &me.into())
            .await;

        // TODO: standardize responses
        return Ok(Json("Friend added".into()));
    }

    // there is no incoming friend request, so we should create an outgoing one
    let rows_affected = sqlx::query!(
        r#"INSERT INTO friend_requests (from_user, to_user) VALUES ($1, $2) ON CONFLICT DO NOTHING"#,
        &me.id,
        &user.id
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
        .notify_friend_request_sent(&user.id, &me.into())
        .await;

    Ok(Json("Friend request sent".into()))
}
