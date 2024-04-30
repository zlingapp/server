use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult, IntoHandlerErrorResult},
    guilds::routes::list_joined_guilds::GuildInfo,
    invites::routes::peek_invite::InviteParams,
    realtime::pubsub::pubsub::PubSub,
};
use actix_web::{
    post,
    web::{Data, Json, Path},
};
use chrono::Utc;
use sqlx::query;

/// Join Guild with Invite
///
/// Redeems an invite code to join a guild, potentially consuming the invite in
/// the process. This counts as a "use" of the invite, which may be limited.
/// 
/// In order to join a guild, the user must have a valid and current invite
/// code. If the invite is expired or out of uses, the request will fail.
/// 
/// Upon success, information about the guild joined is returned, and the
/// user is officially a member of the guild.
#[utoipa::path(
    params(InviteParams),
    responses(
        (status = OK, description = "Guild successfully joined", body = GuildInfo),
        (status = GONE, description = "That invite is expired"),
        (status = CONFLICT, description = "That invite is out of uses"),
        (status = BAD_REQUEST, description = "Invalid invite code"),
    ),
    tag = "invites",
    security(("token" = []))
)]
#[post("/invites/{code}")]
pub async fn use_invite(
    db: DB,
    path: Path<InviteParams>,
    user: User,
    pubsub: Data<PubSub>,
) -> HResult<Json<GuildInfo>> {
    let resp = query!(
        r#"SELECT 
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

    let guild = GuildInfo {
        id: resp.id,
        name: resp.name,
        icon: resp.icon,
    };

    if resp
        .expires_at
        .is_some_and(|dt| dt < Utc::now().naive_utc())
    {
        err!(410, "That invite has expired")?;
    }

    if resp.uses.is_some_and(|uses| uses <= 0) {
        err!(409, "That invite is out of uses")?;
    }

    let mut tx = db.pool.begin().await?;

    // todo: consider setting a field on users to see who they were
    // invited by and when

    let rows_affected = query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2)
        "#,
        &user.id,
        &guild.id
    )
    .execute(&mut tx)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        err!()?;
    }

    if resp.uses.is_some() {
        query!(
            r#"UPDATE invites SET uses = uses - 1 WHERE code = $1"#,
            &path.code
        )
        .execute(&mut tx)
        .await?;
    }

    tx.commit().await?;

    pubsub.notify_guild_member_list_update(&guild.id).await;

    Ok(Json(guild))
}
