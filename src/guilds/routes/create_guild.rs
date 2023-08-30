use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError},
    post,
    web::Json,
};
use log::warn;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{auth::access_token::AccessToken, db::DB, security, channels::channel::ChannelType};

#[derive(Deserialize, ToSchema)]
pub struct CreateGuildRequest {
    #[schema(example = "My Cool Server")]
    name: String,
    #[schema(example = "/api/media/s6NIiu2oOh1FEL0Xfjc7n/cat.jpg")]
    icon: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateGuildResponse {
    #[schema(example = "rMBrzZ7FQk6ZImWlTiRPo")]
    guild_id: String,
}
/// Create Guild
/// 
/// Creates a new guild with the given name and icon. The user will be the owner
/// of the guild.
/// 
/// Default channels will be created for the guild:
/// - A text channel named "general"
/// - A voice channel named "Voice Chat"
/// 
/// The user will be automatically added to the guild as the owner.
#[utoipa::path(
    responses(
        (status = BAD_REQUEST, description = "Invalid icon", example = "invalid_icon"),
        (status = BAD_REQUEST, description = "Invalid guild name", example = "invalid_icon"),
        (status = OK, description = "Guild created successfully", body = CreateGuildResponse)
    ),
    tag = "guilds",
    security(("token" = []))
)]
#[post("/guilds")]
pub async fn create_guild(
    db: DB,
    token: AccessToken,
    req: Json<CreateGuildRequest>,
) -> Result<Json<CreateGuildResponse>, actix_web::Error> {
    let guild_id = nanoid!();

    if let Some(ref icon) = req.icon {
        if !security::validate_resource_origin(icon) {
            return Err(ErrorBadRequest("invalid_icon"));
        }
    }

    // todo: validate guild name

    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|_| ErrorInternalServerError("failed"))?;

    let rows_affected = query_affected(
        sqlx::query!(
            r#"
            INSERT INTO guilds (id, name, owner, icon) 
            SELECT $1, $2, $3, $4
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM guilds WHERE id = $1)
        "#,
            guild_id,
            req.name,
            token.user_id,
            req.icon
        ),
        &mut tx,
    )
    .await
    .map_err(|e| {
        warn!("failed to create guild: {}", e);
        ErrorInternalServerError("failed")
    })?;

    if rows_affected == 0 {
        return Err(ErrorInternalServerError("failed"));
    }

    query_affected(
        sqlx::query!(
            "INSERT INTO members (user_id, guild_id) VALUES ($1, $2)",
            token.user_id,
            guild_id
        ),
        &mut tx,
    )
    .await
    .map_err(|e| {
        warn!(
            "user {} failed to join guild as owner of {}: {}",
            token.user_id, guild_id, e
        );
        ErrorInternalServerError("failed")
    })?;

    query_affected(
        sqlx::query!(
            r#"INSERT INTO channels (guild_id, id, name, type) VALUES ($1, $2, $3, $4)"#,
            guild_id,
            nanoid!(),
            "general",
            ChannelType::Text as ChannelType
        ),
        &mut tx,
    ).await.map_err(|e| {
        warn!(
            "failed to create default text channel when creating guild {}: {}",
            guild_id, e
        );
        ErrorInternalServerError("failed")
    })?;

    query_affected(
        sqlx::query!(
            r#"INSERT INTO channels (guild_id, id, name, type) VALUES ($1, $2, $3, $4)"#,
            guild_id,
            nanoid!(),
            "Voice Chat",
            ChannelType::Voice as ChannelType
        ),
        &mut tx,
    ).await.map_err(|e| {
        warn!(
            "failed to create default voice channel when creating guild {}: {}",
            guild_id, e
        );
        ErrorInternalServerError("failed")
    })?;

    tx.commit()
        .await
        .map_err(|_| ErrorInternalServerError("failed"))?;

    Ok(Json(CreateGuildResponse { guild_id }))
}

/// runs a query on a transaction and returns the rows affected
/// this is only meant to be used here as there's a big repeating code pattern
async fn query_affected(
    query: sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments>,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<u64, sqlx::Error> {
    let rows = query.execute(tx).await?.rows_affected();

    Ok(rows)
}
