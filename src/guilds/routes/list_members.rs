use actix_web::{get, web::Json};

use crate::{
    auth::{access_token::AccessToken, user::PublicUserInfo},
    db::DB,
    error::{macros::err, HResult},
    guilds::routes::{GuildIdParams, GuildPath},
};

/// List Members
///
/// List all members in a guild.
#[utoipa::path(
    params(GuildIdParams),
    responses(
        (status = OK, description = "Success", body = Vec<PublicUserInfo>),
        (status = FORBIDDEN, description = "Access denied")
    ),
    tag = "guilds",
    security(("token" = []))
)]
#[get("/guilds/{guild_id}/members")]
pub async fn list_members(
    db: DB,
    token: AccessToken,
    path: GuildPath,
) -> HResult<Json<Vec<PublicUserInfo>>> {
    let is_in_guild = db.is_user_in_guild(&token.user_id, &path.guild_id).await?;

    if !is_in_guild {
        err!(403)?;
    }

    let members = sqlx::query_as!(
        PublicUserInfo,
        r#"SELECT 
            members.user_id AS "id", 
            users.name AS "username", 
            users.avatar 
        FROM users, members 
        WHERE 
            members.guild_id = $1 
            AND members.user_id = users.id;"#,
        &path.guild_id
    )
    .fetch_all(&db.pool)
    .await?;

    Ok(Json(members))
}
