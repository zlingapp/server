use actix_web::{get, web::Json};

use crate::{
    auth::{access_token::AccessToken, user::PublicUserInfo},
    db::DB,
    error::HResult,
};

/// List Friends
///
/// Lists all users who are friends with you. Users are only considered
/// "friends" when a friend request is fully accepted on both sides.
#[utoipa::path(
    responses(
        (status = OK, description="Friends list", body=Vec<PublicUserInfo>),
    ),
    tag="friends",
    security(("token" = []))
)]
#[get("/friends")]
pub async fn list_friends(db: DB, token: AccessToken) -> HResult<Json<Vec<PublicUserInfo>>> {
    let result = sqlx::query_as!(
        PublicUserInfo,
        r#"SELECT I.id,I.name as "username",I.avatar 
            FROM users AS I 
            JOIN users AS S 
            ON I.id = ANY(S.friends) 
            WHERE S.id = $1"#,
        token.user_id
    )
    .fetch_all(&db.pool)
    .await
    .map(|r| Json(r))?;

    Ok(result)
}
