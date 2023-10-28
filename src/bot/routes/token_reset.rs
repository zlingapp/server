use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError, ErrorNotFound},
    post,
    web::{Json, Path},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::util::use_display;
use crate::{
    auth::{access_token::AccessToken, token::Token},
    bot::routes::delete_bot::BotIdParams,
    db::DB,
};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenResetResponse {
    #[serde(serialize_with = "use_display")]
    refresh_token: Token,
}

/// Reset Bot Token
///
/// Reset your bot's access token.
#[utoipa::path(
    params(BotIdParams),
    responses(
        (status = OK, description = "Success", body = TokenResetResponse),
    ),
    tag = "bots",
    security(("token" = []))
)]
#[post("/bots/{bot_id}/tokenreset")]
pub async fn token_reset(
    db: DB,
    token: AccessToken,
    req: Path<BotIdParams>,
) -> Result<Json<TokenResetResponse>, actix_web::Error> {
    if token.is_bot() {
        return Err(ErrorForbidden("bot_access_denied"));
    }

    // check if bot exists and is owned by user
    let bot = sqlx::query!(
        r#"
            SELECT owner_id FROM bots WHERE id = $1;
        "#,
        req.bot_id
    )
    .fetch_optional(&db.pool)
    .await
    .map_err(|_| ErrorInternalServerError(""))?;

    if let Some(bot) = bot {
        if bot.owner_id != token.user_id {
            return Err(ErrorForbidden("not_bot_owner"));
        }
    } else {
        return Err(ErrorNotFound("bot_not_found"));
    }

    // delete old token
    sqlx::query!(
        r#"
            DELETE FROM tokens WHERE user_id = $1;
        "#,
        req.bot_id
    )
    .execute(&db.pool)
    .await
    .map_err(|_| ErrorInternalServerError(""))?;

    // create new token
    let refresh_token = db
        .create_refresh_token(&req.bot_id, "zling-bot", true)
        .await;

    Ok(Json(TokenResetResponse { refresh_token }))
}
