use actix_web::{delete, HttpResponse};

use crate::{auth::access_token::AccessToken, db::DB, guilds::routes::GuildPath};
use crate::{
    error::{macros::err, HResult},
    guilds::routes::GuildIdParams,
};

/// Delete a Guild
///
/// This endpoint requires the user to be the owner of the guild.
#[utoipa::path(
    params(GuildIdParams),
    responses(
        (status = FORBIDDEN, description = "Not the owner of the guild", example = "access_denied"),
        (status = OK, description = "Guild deleted successfully", example = "success")
    ),
    tag = "guilds",
    security(("token" = []))
)]
#[delete("/guilds/{guild_id}")]
pub async fn delete_guild(db: DB, token: AccessToken, req: GuildPath) -> HResult<HttpResponse> {
    let rows_affected = sqlx::query!(
        r#"
            DELETE FROM guilds WHERE id = $1 AND owner = $2
        "#,
        req.guild_id,
        token.user_id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        err!()?;
    }

    Ok(HttpResponse::Ok().body("success"))
}
