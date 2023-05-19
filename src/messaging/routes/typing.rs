use actix_web::{
    error::ErrorUnauthorized,
    error::Error,
    post,
    web::{Data, Path},
    HttpResponse,
};

use crate::{auth::user::UserEx, db::DB, realtime::pubsub::consumer_manager::EventConsumerManager};

#[post("/guilds/{guild_id}/channels/{channel_id}/typing")]
pub async fn typing(
    db: DB,
    path: Path<(String, String)>,
    user: UserEx,
    ecm: Data<EventConsumerManager>,
) -> Result<HttpResponse, Error> {
    let (guild_id, channel_id) = path.into_inner();

    if !db
        .can_user_send_message_in(&guild_id, &user.id, &channel_id)
        .await
        .unwrap()
    {
        return Err(ErrorUnauthorized("access_denied"))
    }

    ecm.send_typing(&channel_id, &user).await;

    Ok(HttpResponse::Ok().finish())
}
