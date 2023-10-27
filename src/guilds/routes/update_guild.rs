use actix_web::{put, HttpResponse};

use crate::{auth::access_token::AccessToken, db::DB, error::HResult, guilds::routes::GuildPath};

#[put("/guilds/{guild_id}")]
pub async fn update_guild(_db: DB, _token: AccessToken, _req: GuildPath) -> HResult<HttpResponse> {
    // todo: do this
    Ok(HttpResponse::NotImplemented().body("not_implemented"))
}
