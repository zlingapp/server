use actix_web::{get, web::Json};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::{schema, ToSchema};

use crate::{
    auth::{access_token::AccessToken, user::PublicUserInfo},
    db::DB,
    error::{macros::err, HResult},
    guilds::routes::{GuildIdParams, GuildPath},
};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteInfo {
    #[schema(example = "7UU0KB41")]
    pub code: String,
    #[schema(example = 10)]
    pub uses_remaining: Option<i32>,
    pub expiry: Option<DateTime<Utc>>,
    pub creator: PublicUserInfo,
}

/// List Invites
///
/// Lists all active invites for a guild.
#[utoipa::path(
    params(GuildIdParams),
    responses(
        (status = OK, description = "Success", body = Vec<InviteInfo>),
        (status = FORBIDDEN, description = "Access denied")
    ),
    tag = "invites",
    security(("token" = []))
)]
#[get("/guilds/{guild_id}/invites")]
pub async fn list_invites(
    db: DB,
    token: AccessToken,
    path: GuildPath,
) -> HResult<Json<Vec<InviteInfo>>> {
    let is_in_guild = db.is_user_in_guild(&token.user_id, &path.guild_id).await?;

    if !is_in_guild {
        err!(403)?;
    }

    let invites = sqlx::query!(
        r#"SELECT 
            invites.code, 
            invites.uses, 
            invites.creator, 
            invites.expires_at,
            users.id,
            users.name,
            users.avatar
        FROM
            invites, users
        WHERE 
            invites.guild_id = $1
        AND invites.creator = users.id;"#,
        &path.guild_id
    )
    .fetch_all(&db.pool)
    .await?
    .into_iter()
    .map(|row| InviteInfo {
        code: row.code,
        uses_remaining: row.uses,
        expiry: row
            .expires_at
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)),
        creator: PublicUserInfo {
            id: row.id,
            username: row.name,
            avatar: row.avatar,
        },
    })
    .collect();

    Ok(Json(invites))
}
