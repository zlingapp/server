use actix_web::{put, HttpResponse};

use crate::{auth::token::TokenEx, db::DB, guilds::routes::GuildPath};

#[put("/guilds/{guild_id}")]
pub async fn update_guild(
    _db: DB,
    _token: TokenEx,
    _req: GuildPath,
) -> Result<HttpResponse, actix_web::Error> {
    // todo: do this
    Ok(HttpResponse::NotImplemented().body("not_implemented"))
}
