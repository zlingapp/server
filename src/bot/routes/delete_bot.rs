use actix_web::{
    delete,
    error::{ErrorForbidden, ErrorInternalServerError, ErrorNotFound},
    web::Path,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{auth::access_token::AccessToken, db::DB};

#[derive(Deserialize, IntoParams)]
pub struct BotIdParams {
    pub bot_id: String,
}

/// Delete Bot
///
/// Deletes a bot you own.
#[utoipa::path(
    params(BotIdParams),
    responses(
        (status = OK, description = "Success"),
    ),
    tag = "bots",
    security(("token" = []))
)]
#[delete("/bots/{bot_id}")]
pub async fn delete_bot(
    db: DB,
    token: AccessToken,
    req: Path<BotIdParams>,
) -> Result<&'static str, actix_web::Error> {
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

    // start transaction
    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|_| ErrorInternalServerError(""))?;

    sqlx::query!(
        r#"
            DELETE FROM bots WHERE id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await
    .map_err(|_| ErrorInternalServerError(""))?;

    sqlx::query!(
        r#"
            DELETE FROM users WHERE id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await
    .map_err(|_| ErrorInternalServerError(""))?;

    sqlx::query!(
        r#"
            DELETE FROM tokens WHERE user_id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await
    .map_err(|_| ErrorInternalServerError(""))?;

    sqlx::query!(
        r#"
            DELETE FROM members WHERE user_id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await
    .map_err(|_| ErrorInternalServerError(""))?;

    tx.commit()
        .await
        .map_err(|_| ErrorInternalServerError(""))?;

    Ok("success")
}
