use crate::DB;

pub async fn is_user_in_guild(db: &DB, user_id: &str, guild_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT EXISTS (SELECT 1 FROM members WHERE guild_id = $1 AND user_id = $2) AS user_in_guild",
        guild_id,
        user_id
    )
    .fetch_one(db.as_ref())
    .await?
    .user_in_guild.unwrap();

    Ok(result)
}

pub async fn can_user_see_channel(
    db: &DB,
    user_id: &str,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "
        SELECT EXISTS (
            SELECT 1 
                FROM members, channels 
                WHERE (
                    channels.id = $1        AND 
                    members.user_id = $2    AND 
                    channels.guild_id = members.guild_id
                )
        ) AS can_user_see_channel
        ",
        channel_id,
        user_id
    )
    .fetch_one(db.as_ref())
    .await?
    .can_user_see_channel
    .unwrap();

    Ok(result)
}

pub async fn can_user_send_message_in(db: &DB, user_id: &str, channel_id: &str) -> Result<bool, sqlx::Error> {
    return can_user_see_channel(db, user_id, channel_id).await;
}

pub async fn can_user_read_message_history_from(db: &DB, user_id: &str, channel_id: &str) -> Result<bool, sqlx::Error> {
    return can_user_see_channel(db, user_id, channel_id).await;
}