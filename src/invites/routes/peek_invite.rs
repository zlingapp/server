use crate::{
    db::DB,
    error::{macros::err, HResult, IntoHandlerErrorResult},
    guilds::routes::list_joined_guilds::GuildInfo,
};
use actix_web::{
    get,
    web::{Json, Path},
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::query;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
pub struct InviteParams {
    pub code: String,
}

/// View Invite
///
/// Retrieve info about a guild referred to by an invite without consuming a
/// use. Effectively "peeks" at the invite's guild without joining it.
#[utoipa::path(
    params(InviteParams),
    responses(
        (status = OK, description = "Guild information", body = GuildInfo),
        (status = GONE, description = "That invite is expired"),
        (status = CONFLICT, description = "That invite is out of uses"),
        (status = BAD_REQUEST, description = "Invalid invite code"),
    ),
    tag = "invites",
    security(("token" = []))
)]
#[get("/invites/{code}")]
pub async fn peek_invite(db: DB, path: Path<InviteParams>) -> HResult<Json<GuildInfo>> {
    let resp = query!(
        r#"
            SELECT 
                guilds.id, guilds.name, guilds.icon, 
                invites.expires_at, invites.uses
            FROM 
                guilds, invites
            WHERE 
                invites.code = $1
            AND 
                invites.guild_id = guilds.id
        "#,
        path.code
    )
    .fetch_optional(&db.pool)
    .await?
    .or_err_msg(400, "Invalid invite code")?;

    if resp
        .expires_at
        .is_some_and(|dt| dt < Utc::now().naive_utc())
    {
        err!(410, "That invite is expired")?;
    }

    if resp.uses.is_some_and(|uses| uses <= 0) {
        err!(409, "That invite is out of uses")?;
    }

    let guild = GuildInfo {
        id: resp.id,
        name: resp.name,
        icon: resp.icon,
    };

    Ok(Json(guild))
}
