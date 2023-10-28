use actix_web::{delete, web::Path};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    auth::access_token::AccessToken,
    db::DB,
    error::{macros::err, HResult},
};

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
) -> HResult<&'static str> {
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

    // start transaction
    let mut tx = db.pool.begin().await?;

    sqlx::query!(
        r#"
            DELETE FROM bots WHERE id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        r#"
            DELETE FROM users WHERE id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        r#"
            DELETE FROM tokens WHERE user_id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        r#"
            DELETE FROM members WHERE user_id = $1;
        "#,
        req.bot_id
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok("success")
}
