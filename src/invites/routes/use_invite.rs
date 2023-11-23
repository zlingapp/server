use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    invites::routes::see_invite::InvitePath,
    realtime::pubsub::pubsub::PubSub,
};
use actix_web::{
    post,
    web::{Data, Json, Path},
};
use chrono::Utc;
use sqlx::query;

/// Use an invite code
///
/// Redeems an invite code, joining the associated guild
#[utoipa::path(
    params(InvitePath),
    responses(
        (status = OK, description = "Guild successfully joined", body = GuildInfo),
        (status = GONE, description = "That invite is expired"),
        (status = GONE, description = "That invite is out of uses"),
        (status = BAD_REQUEST, description = "Invalid invite code"),
    ),
    tag = "invites",
    security(("token" = []))
)]
#[post("/invites/{invite_id}")]
pub async fn use_invite(
    db: DB,
    path: Path<InvitePath>,
    user: User,
    pubsub: Data<PubSub>,
) -> HResult<Json<String>> {
    let resp = query!(
        r#"SELECT guilds.id, invites.expires_at, invites.uses
            FROM guilds, invites
            WHERE invites.code = $1
            AND invites.guild_id = guild_id"#,
        path.invite_id
    )
    .fetch_optional(&db.pool)
    .await?
    .unwrap_or(err!(400, "Invalid invite code")?);

    if resp
        .expires_at
        .is_some_and(|dt| dt < Utc::now().naive_utc())
    {
        err!(410, "That invite is expired")?;
    }
    if resp.uses.is_some_and(|uses| uses <= 0) {
        err!(410, "That invite is out of uses")?;
    }

    let rows_affected = query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2) 
            AND EXISTS (SELECT 1 FROM guilds WHERE guilds.id = $2)
        "#,
        user.id,
        resp.id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        err!()?;
    }
    if let Some(uses) = resp.uses {
        query!(
            r#"UPDATE invites
                    SET uses = $1
                    WHERE code = $2"#,
            uses - 1,
            &path.invite_id
        )
        .execute(&db.pool)
        .await?;
    }
    pubsub.notify_guild_member_list_update(&resp.id).await;

    // TODO standardised responses
    Ok(Json("Guild successfully joined".into()))
}
