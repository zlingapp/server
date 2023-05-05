use actix_web::{put, HttpResponse};

use crate::{db::DB, auth::user::UserEx, guilds::routes::GuildPath};

#[put("/guilds/{guild_id}")]
pub async fn update_guild(_db: DB, _user: UserEx, _req: GuildPath) -> Result<HttpResponse, actix_web::Error> {
    // todo: do this
    Ok(HttpResponse::NotImplemented().body("not_implemented"))
}
