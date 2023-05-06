use actix_web::{
    error::{ErrorConflict, ErrorInternalServerError},
    post,
    web::Json,
};
use log::warn;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use crate::{auth::access_token::AccessToken, db::DB};

#[derive(Deserialize)]
pub struct CreateGuildQuery {
    name: String,
}

#[derive(Serialize)]
pub struct CreateGuildResponse {
    guild_id: String,
}

#[post("/guilds")]
pub async fn create_guild(
    db: DB,
    token: AccessToken,
    req: Json<CreateGuildQuery>,
) -> Result<Json<CreateGuildResponse>, actix_web::Error> {
    let guild_id = nanoid!();

    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|_| ErrorInternalServerError("failed"))?;

    // create the guild
    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO guilds (id, name, owner) 
            SELECT $1, $2, $3
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM guilds WHERE id = $1)
        "#,
        guild_id,
        req.name,
        token.user_id
    )
    .execute(&mut tx)
    .await
    .map_err(|e| {
        warn!("failed to create guild: {}", e);
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorConflict("guild_create_id_conflict"));
    }

    // add owner as member to guild

    // todo: move this to db.user_join_guild()
    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2)
        "#,
        token.user_id,
        guild_id
    )
    .execute(&mut tx)
    .await
    .map_err(|e| {
        warn!(
            "user {} failed to join guild as OWNER {}: {}",
            token.user_id, guild_id, e
        );
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorConflict("owner_join_id_conflict"));
    }

    tx.commit()
        .await
        .map_err(|_| ErrorInternalServerError("failed"))?;

    Ok(Json(CreateGuildResponse { guild_id }))
}
