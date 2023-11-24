use crate::{
    auth::user::User,
    db::DB,
    error::{macros::err, HResult},
};
use actix_web::{post, web::Json};
use chrono::{DateTime, Utc};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::query;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateInviteRequest {
    #[schema(example = "gbMs1hs7c8IZBDd34_c_1")]
    pub guild_id: String,
    #[schema(example = 10)]
    pub uses: Option<i32>,
    #[schema(example = "2024-04-20T00:00:00.000Z")]
    pub expires_at: Option<DateTime<Utc>>,
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

/// Create an invite code
///
/// Creates an invite code with a certain expiry and amount of uses for a certain guild.
///
/// This code can then be used to gain info about the guild, or to join the guild, redeeming a use.
///
/// Setting expires_at to null represents an indefinite invite, and setting uses to null represents unlimited uses.
#[utoipa::path(
    params(),
    responses(
        (status = OK, description = "Invite successfully created", body = CreateInviteResponse),
        (status = FORBIDDEN, description = "No permission to create an invite for that guild"),
        (status = BAD_REQUEST, description = "Can't create an invite that expires in the past"),
        (status = BAD_REQUEST, description = "Invites must have a positive or null amount of uses")
    ),
    tag = "invites",
    security(("token" = []))
)]
#[post("/invites/create")]
pub async fn create_invite(
    db: DB,
    req: Json<CreateInviteRequest>,
    user: User,
) -> HResult<Json<CreateInviteResponse>> {
    if !db
        .can_user_create_invite_in(&user.id, &req.guild_id)
        .await
        .unwrap_or(false)
    // This probably errors if the guild doesn't exist
    {
        err!(403)?;
    }

    if req.expires_at.is_some_and(|x| Utc::now() > x) {
        // TODO Standard responses
        err!(400, "Can't create invite that expires in the past")?;
    }

    if req.uses.is_some_and(|x| x <= 0) {
        err!(400, "An invite needs a positive (or null) number of uses")?;
    }
    // Might be unnecessary? See foreign key constraints, above 403 check
    // if query!("SELECT name FROM guilds WHERE id = $1", req.guild_id)
    //     .fetch_optional(&db.pool)
    //     .await?
    //     .is_none()
    // {
    //     err!(400, "Invalid guild id")?;
    // }

    let code = query!(
        r#"INSERT INTO invites (code, guild_id, inviter, uses, expires_at) 
            VALUES ($1,$2,$3,$4,$5) 
            RETURNING code"#,
        nanoid!(INVITE_CODE_LENGTH, &INVITE_CODE_ALPHABET),
        &req.guild_id,
        &user.id,
        req.uses,
        req.expires_at.map(|x| x.naive_utc())
    )
    .fetch_one(&db.pool)
    .await?;

    Ok(Json(CreateInviteResponse { code: code.code }))
}
