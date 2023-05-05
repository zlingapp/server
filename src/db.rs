use actix_web::web::Data;
use sqlx::query;
use sqlx::{Pool, Postgres};

use crate::auth::user::User;
use crate::crypto;

pub type DB = Data<Database>;

pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
    pub fn with_pool(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn register_user(&self, user: &User, password: &str) -> Result<bool, sqlx::Error> {
        let rows_affected = query!(
            r#"
                INSERT INTO users (id, name, email, avatar, password) 
                SELECT $1, $2, $3, $4, $5 
                FROM (SELECT 1) AS t
                WHERE NOT EXISTS (SELECT 1 FROM users WHERE email = $3)
            "#,
            user.id,
            user.name,
            user.email,
            user.avatar,
            crypto::hash(password)
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn is_user_in_guild(
        &self,
        user_id: &str,
        guild_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT EXISTS (SELECT 1 FROM members WHERE guild_id = $1 AND user_id = $2) AS user_in_guild",
            guild_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .user_in_guild.unwrap();

        Ok(result)
    }

    pub async fn can_user_see_channel(
        &self,
        guild_id: &str,
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
                        channels.guild_id = $3  AND
                        members.guild_id = $3
                    )
            ) AS can_user_see_channel
            ",
            channel_id,
            user_id,
            guild_id
        )
        .fetch_one(&self.pool)
        .await?
        .can_user_see_channel
        .unwrap();

        Ok(result)
    }

    pub async fn can_user_send_message_in(
        &self,
        guild_id: &str,
        user_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        return self.can_user_see_channel(guild_id, user_id, channel_id).await;
    }

    pub async fn can_user_read_message_history_from(
        &self,
        guild_id: &str,
        user_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        return self.can_user_see_channel(guild_id, user_id, channel_id).await;
    }
}
