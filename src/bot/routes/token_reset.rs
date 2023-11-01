use actix_web::{
    post,
    web::{Json, Path},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::{macros::err, HResult};
use crate::{
    auth::{access_token::AccessToken, token::Token},
    bot::routes::delete_bot::BotIdParams,
    db::DB,
};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenResetResponse {
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
) -> HResult<Json<TokenResetResponse>> {
    if token.is_bot() {
        err!(403, "Bot access disallowed")?;
    }

    // check if bot exists and is owned by user
    let bot = sqlx::query!(
        r#"
            SELECT owner_id FROM bots WHERE id = $1;
        "#,
        req.bot_id
    )
    .fetch_optional(&db.pool)
    .await?;

    if let Some(bot) = bot {
        if bot.owner_id != token.user_id {
            err!(403, "You are not the owner of this bot.")?;
        }
    } else {
        err!(404, "A bot with that ID does not exist.")?;
    }

    // delete old token
    sqlx::query!(
        r#"
            DELETE FROM tokens WHERE user_id = $1;
        "#,
        req.bot_id
    )
    .execute(&db.pool)
    .await?;

    // create new token
    let refresh_token = db
        .create_refresh_token(&req.bot_id, "zling-bot", true)
        .await;

    Ok(Json(TokenResetResponse { refresh_token }))
}
