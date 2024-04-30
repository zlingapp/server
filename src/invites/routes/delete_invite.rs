use crate::invites::routes::peek_invite::InviteParams;
use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
};
use actix_web::{
    delete,
    web::{Json, Path},
};
use sqlx::query;

/// Delete Invite
///
/// Deletes a guild invite.
#[utoipa::path(
    params(InviteParams),
    responses(
        (status = OK, description = "Invite successfully deleted"),
        (status = FORBIDDEN, description = "No permission to delete invite"),
    ),
    tag = "invites",
    security(("token" = []))
)]
#[delete("/invites/{code}")]
pub async fn delete_invite(db: DB, path: Path<InviteParams>, user: User) -> HResult<Json<String>> {
    if db
        .can_user_delete_invite(&user.id, &path.code)
        .await
        .unwrap_or(false)
    {
        err!(403)?;
    }
    
    let rows_affected = query!("DELETE FROM invites WHERE code = $1", path.code)
        .execute(&db.pool)
        .await?
        .rows_affected();

    if rows_affected == 0 {
        err!()?;
    }

    Ok(Json("Invite successfully deleted".into()))
}
