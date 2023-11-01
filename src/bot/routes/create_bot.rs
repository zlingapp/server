use actix_web::{post, web::Json};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::routes::register::{generate_discrim, USERNAME_REGEX};
use crate::auth::user::PublicUserInfo;
use crate::error::macros::err;
use crate::error::HResult;
use crate::{
    auth::{access_token::AccessToken, token::Token},
    db::DB,
};

#[derive(Deserialize, ToSchema)]
pub struct CreateBotRequest {
    #[schema(example = "zling")]
    username: String,
    avatar: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BotDetails {
    pub user: PublicUserInfo,
    pub refresh_token: Token,
}

/// Create Bot
///
/// Create a bot application.
#[utoipa::path(
    responses(
        (status = OK, description = "Success", body = BotDetails),
    ),
    tag = "bots",
    security(("token" = []))
)]
#[post("/bots")]
pub async fn create_bot(
    db: DB,
    token: AccessToken,
    req: Json<CreateBotRequest>,
) -> HResult<Json<BotDetails>> {
    if token.is_bot() {
        err!(403, "Bot access disallowed")?;
    }

    // check if name is ascii and alphanumeric
    if !USERNAME_REGEX.is_match(&req.username) {
        err!(400, "Invalid username")?;
    }

    // todo: better check here
    if !req.avatar.starts_with("/media/") {
        err!(400, "Invalid avatar")?;
    }

    let bot_user_id = format!("bot:{}", nanoid!());
    let bot_name = format!("{}#{}", req.username, generate_discrim());

    let rows_affected = sqlx::query!(
        r#"
            INSERT INTO users (id, name, avatar, bot) 
            SELECT $1, $2, $3, true
            FROM (SELECT 1) AS t
            WHERE NOT EXISTS (SELECT 1 FROM users WHERE name = $2)
        "#,
        bot_user_id,
        bot_name,
        req.avatar,
    )
    .execute(&db.pool)
    .await?
    .rows_affected();

    if rows_affected != 1 {
        err!(409, "That username is already taken.")?;
    }

    sqlx::query!(
        r#"INSERT INTO bots (id, owner_id) VALUES ($1, $2);"#,
        bot_user_id,
        token.user_id,
    )
    .execute(&db.pool)
    .await?;

    let refresh_token = db
        .create_refresh_token(&bot_user_id, "zling-bot", true)
        .await;

    let user = PublicUserInfo {
        id: bot_user_id,
        username: bot_name,
        avatar: req.avatar.clone(),
    };

    Ok(Json(BotDetails {
        user,
        refresh_token,
    }))
}
