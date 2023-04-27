/*

   - List channels in a guild
   - Get channel history
   - Send to a channel
   - Create a channel
   - Delete a channel

*/

use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    get,
    web::{Data, Json, Query, self},
    Error, HttpResponse,
};
use log::warn;
use serde::Serialize;

use crate::{
    auth::user::{UserEx, UserManager},
    guilds::handlers::GuildIdQuery,
    DB,
};

pub fn scope() -> actix_web::Scope {
    web::scope("/channels").service(list_channels)
}

#[derive(sqlx::Type, Serialize, Debug)]
#[sqlx(type_name = "channel_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}
#[derive(Serialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub r#type: ChannelType,
}

#[get("/list")]
async fn list_channels(
    db: DB,
    um: Data<UserManager>,
    user: UserEx,
    g_query: Query<GuildIdQuery>,
) -> Result<Json<Vec<ChannelInfo>>, Error> {
    let user_in_guild = um
        .is_user_in_guild(&user.id, &g_query.id)
        .await
        .map_err(|e| {
            warn!(
                "failed to check if user {} is in guild {}: {}",
                user.id, g_query.id, e
            );
            ErrorInternalServerError("")
        })?;

    if !user_in_guild {
        return Err(ErrorUnauthorized("access_denied"));
    }

    let channels = sqlx::query_as!(
        ChannelInfo,
        r#"SELECT id, type AS "type: _", name FROM channels WHERE guild_id = $1"#,
        g_query.id
    )
    .fetch_all(db.as_ref())
    .await
    .map_err(|e| {
        warn!(
            "failed to retreive channels for guild {}: {}",
            g_query.id, e
        );
        ErrorInternalServerError("")
    })?;

    Ok(Json(channels))
}
