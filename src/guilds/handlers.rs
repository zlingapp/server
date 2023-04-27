/*

    - List my guilds ( query members table for guilds where user_id = current user id )
    - Create a guild
    - Delete a guild
    - Update a guild's name, etc.

*/

use actix_web::{
    error::{ErrorConflict, ErrorInternalServerError, ErrorUnauthorized},
    get, post,
    web::{self, Json, Query},
    HttpResponse,
};
use log::warn;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{auth::user::UserEx, DB};

pub fn scope() -> actix_web::Scope {
    web::scope("/guilds")
        .service(create_guild)
        .service(delete_guild)
        .service(list_guilds)
        .service(join_guild)
}

#[derive(Deserialize)]
pub struct CreateGuildQuery {
    name: String,
}

#[post("/create")]
pub async fn create_guild(
    db: DB,
    user: UserEx,
    req_query: Query<CreateGuildQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let guild_id = nanoid!();

    let mut tx = db
        .begin()
        .await
        .map_err(|_| ErrorInternalServerError("failed"))?;

    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO guilds (id, name, owner) 
            SELECT $1, $2, $3
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM guilds WHERE id = $1)
        "#,
        guild_id,
        req_query.name,
        user.id
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

    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2)
        "#,
        user.id,
        guild_id
    )
    .execute(&mut tx)
    .await
    .map_err(|e| {
        warn!(
            "user {} failed to join guild as OWNER {}: {}",
            user.id, guild_id, e
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

    Ok(HttpResponse::Ok().json(json!({ "guild_id": guild_id })))
}

#[derive(Deserialize)]
pub struct GuildIdQuery {
    pub id: String,
}

#[post("/delete")]
pub async fn delete_guild(
    db: DB,
    user: UserEx,
    guild_query: Query<GuildIdQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let rows_affected = sqlx::query!(
        r#"
            DELETE FROM guilds WHERE id = $1 AND owner = $2
        "#,
        guild_query.id,
        user.id
    )
    .execute(db.as_ref())
    .await
    .map_err(|e| {
        warn!("failed to delete guild: {}", e);
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorUnauthorized("access_denied"));
    }

    Ok(HttpResponse::Ok().body("success"))
}

#[derive(Serialize)]
pub struct GuildNameAndId {
    id: String,
    name: String,
}

#[get("/list")]
pub async fn list_guilds(
    db: DB,
    user: UserEx,
) -> Result<Json<Vec<GuildNameAndId>>, actix_web::Error> {
    let guilds_list = sqlx::query_as!(
        GuildNameAndId,
        r#"
            SELECT members.guild_id AS "id", guilds.name FROM members, guilds 
            WHERE members.user_id = $1 AND members.guild_id = guilds.id
        "#,
        user.id
    )
    .fetch_all(db.as_ref())
    .await
    .map_err(|e| {
        warn!("failed to list guilds for user {}: {}", user.id, e);
        ErrorInternalServerError("failed")
    })?;

    Ok(Json(guilds_list))
}

#[get("/join")]
pub async fn join_guild(
    db: DB,
    user: UserEx,
    guild_query: Query<GuildIdQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO members (user_id, guild_id) 
            SELECT $1, $2
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2)
        "#,
        user.id,
        guild_query.id
    )
    .execute(db.as_ref())
    .await
    .map_err(|e| {
        warn!(
            "user {} failed to join guild {}: {}",
            user.id, guild_query.id, e
        );
        ErrorInternalServerError("failed")
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ErrorConflict("id_conflict"));
    }

    Ok(HttpResponse::Ok().body("success"))
}

#[post("/update")]
pub async fn update_guild(db: DB, user: UserEx) -> Result<HttpResponse, actix_web::Error> {
    // todo: do this
    Ok(HttpResponse::NotImplemented().body("not_implemented"))
}
