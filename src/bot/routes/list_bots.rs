use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    get,
    web::Json,
};
use chrono::Utc;
use log::error;

use crate::{
    auth::{access_token::AccessToken, token::Token, user::PublicUserInfo},
    bot::routes::create_bot::BotDetails,
    db::DB,
};

/// List Bots
///
/// List all bots you have created.
#[utoipa::path(
    responses(
        (status = OK, description = "Success", body = Vec<BotDetails>),
    ),
    tag = "bots",
    security(("token" = []))
)]
#[get("/bots")]
pub async fn list_bots(
    db: DB,
    token: AccessToken,
) -> Result<Json<Vec<BotDetails>>, actix_web::Error> {
    if token.is_bot() {
        return Err(ErrorForbidden("bot_access_denied"));
    }

    let rows = sqlx::query!(
        r#"
        SELECT users.id, users.name, users.avatar, tokens.nonce, tokens.expires_at
        FROM bots, users, tokens WHERE bots.owner_id = $1 AND users.id = bots.id AND tokens.user_id = bots.id;"#,
        token.user_id
    ).fetch_all(&db.pool).await.map_err(|e| {
        error!("Failed to fetch bots: {}", e);
        ErrorInternalServerError("")
    })?;

    let details = rows
        .iter()
        .map(|row| BotDetails {
            user: PublicUserInfo {
                id: row.id.clone(),
                username: row.name.clone(),
                avatar: row.avatar.clone(),
            },
            refresh_token: Token {
                user_id: row.id.clone(),
                expires: row.expires_at.and_local_timezone(Utc).single().unwrap(),
                proof: row.nonce.clone(),
            },
        })
        .collect();

    Ok(Json(details))
}
