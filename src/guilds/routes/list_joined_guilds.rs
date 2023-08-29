use actix_web::{error::ErrorInternalServerError, get, web::Json};
use log::warn;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{auth::access_token::AccessToken, db::DB};

#[derive(Serialize, ToSchema)]
pub struct GuildInfo {
    #[schema(example = "rMBrzZ7FQk6ZImWlTiRPo")]
    id: String,
    #[schema(example = "My Cool Server")]
    name: String,
    #[schema(example = "/api/media/s6NIiu2oOh1FEL0Xfjc7n/cat.jpg")]
    icon: Option<String>,
}

/// List Joined Guilds
/// 
/// List all guilds that the user is a member of. This is used to populate the
/// guild list on the client for the first time.
/// 
/// Returned information is limited to the guild ID, name, and icon.
#[utoipa::path(
    responses(
        (status = OK, description = "Guild list", body = Vec<GuildInfo>)
    ),
    tag = "guilds",
    security(("token" = []))
)]
#[get("/guilds")]
pub async fn list_joined_guilds(
    db: DB,
    token: AccessToken,
) -> Result<Json<Vec<GuildInfo>>, actix_web::Error> {
    let guilds_list = sqlx::query_as!(
        GuildInfo,
        r#"
            SELECT guilds.id, guilds.name, guilds.icon FROM members, guilds 
            WHERE members.user_id = $1 AND members.guild_id = guilds.id
        "#,
        token.user_id
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        warn!("failed to list guilds for user {}: {}", token.user_id, e);
        ErrorInternalServerError("failed")
    })?;

    Ok(Json(guilds_list))
}
