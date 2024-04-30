use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
    guilds::routes::{GuildIdParams, GuildPath},
};
use actix_web::{post, web::Json};
use chrono::{DateTime, Utc};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::query_as;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateInviteRequest {
    #[schema(example = 10)]
    pub max_uses: Option<i32>,
    #[schema(example = "2024-04-20T00:00:00.000Z")]
    pub expiry: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateInviteResponse {
    #[schema(example = "7UU0KB41")]
    pub code: String,
}

const INVITE_CODE_LENGTH: usize = 8;
const INVITE_CODE_ALPHABET: [char; 34] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
];

/// Create Invite
///
/// Creates an invite code with a certain expiry and amount of uses for a
/// certain guild.
///
/// This code can then be used to gain info about the guild, or to join the
/// guild, redeeming a use.
///
/// Setting `expiry` to null represents an invite with no expiry, and
/// setting `max_uses` to null represents unlimited uses.
#[utoipa::path(
    params(GuildIdParams),
    responses(
        (status = OK, description = "Invite successfully created", body = CreateInviteResponse),
        (status = FORBIDDEN, description = "No permission to create an invite for that guild"),
        (status = BAD_REQUEST, description = "Invalid expiry or uses value requested"),
    ),
    tag = "invites",
    security(("token" = []))
)]
#[post("/guilds/{guild_id}/invites")]
pub async fn create_invite(
    db: DB,
    path: GuildPath,
    req: Json<CreateInviteRequest>,
    user: User,
) -> HResult<Json<CreateInviteResponse>> {
    if !db
        .can_user_create_invite_in(&user.id, &path.guild_id)
        .await?
    {
        err!(403)?;
    }

    if req.expiry.is_some_and(|x| Utc::now() > x) {
        // TODO Standard responses
        err!(400, "Can't create invite that expires in the past")?;
    }

    if req.max_uses.is_some_and(|x| x <= 0) {
        err!(400, "An invite needs a positive (or null) number of uses")?;
    }

    let resp = query_as!(
        CreateInviteResponse,
        r#"INSERT INTO invites (code, guild_id, creator, uses, expires_at) 
            VALUES ($1, $2, $3, $4, $5) 
            RETURNING code"#,
        nanoid!(INVITE_CODE_LENGTH, &INVITE_CODE_ALPHABET),
        &path.guild_id,
        &user.id,
        req.max_uses,
        req.expiry.map(|x| x.naive_utc())
    )
    .fetch_one(&db.pool)
    .await?;

    Ok(Json(resp))
}
