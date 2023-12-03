use crate::invites::routes::see_invite::InvitePath;
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

/// Delete an invite
///
/// Deletes a guild invite. Currently requires user to be owner of the invite guild.
#[utoipa::path(
    params(InvitePath),
    responses(
        (status = OK, description = "Invite successfully deleted"),
        (status = FORBIDDEN, description = "No permission to delete an invite for that guild"),
        (status = FORBIDDEN, description = "No such invite exists")
    ),
    tag = "invites",
    security(("token" = []))
)]
#[delete("/invites/{invite_id}")]
pub async fn delete_invite(db: DB, path: Path<InvitePath>, user: User) -> HResult<Json<String>> {
    if db
        .can_user_delete_invite(&user.id, &path.invite_id)
        .await
        .unwrap_or(false)
    {
        err!(403)?;
    }
    let rows_affected = query!("DELETE FROM invites WHERE code = $1", path.invite_id)
        .execute(&db.pool)
        .await?
        .rows_affected();
    if rows_affected == 0 {
        err!()?;
    }
    Ok(Json("Invite successfully deleted".into()))
}
