use actix_web::web::Data;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use sqlx::query;
use sqlx::{Pool, Postgres};

use crate::auth::user::{PublicUserInfo, User};
use crate::crypto;
use crate::messaging::message::Message;

pub type DB = Data<Database>;

pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
    pub fn with_pool(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
                SELECT id, name, email, avatar, bot
                FROM users
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn register_user(&self, user: &User, password: &str) -> Result<bool, sqlx::Error> {
        let rows_affected = query!(
            r#"
                INSERT INTO users (id, name, email, avatar, password, bot) 
                SELECT $1, $2, $3, $4, $5, $6
                FROM (SELECT 1) AS t
                WHERE NOT EXISTS (SELECT 1 FROM users WHERE email = $3 OR name = $2)
            "#,
            user.id,
            user.name,
            user.email,
            user.avatar,
            crypto::hash(password),
            user.bot
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
        user_id: &str,
        guild_id: &str,
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
        user_id: &str,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        return self
            .can_user_see_channel(user_id, guild_id, channel_id)
            .await;
    }

    pub async fn can_user_read_message_history_from(
        &self,
        user_id: &str,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        return self
            .can_user_see_channel(user_id, guild_id, channel_id)
            .await;
    }

    /// this is different to message history! this is the ability to read messages
    /// even if they have been sent after the user has joined the channel.
    /// if this is false, it takes priority over "read message history".
    pub async fn can_user_view_messages_in(
        &self,
        user_id: &str,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        return self
            .can_user_see_channel(user_id, guild_id, channel_id)
            .await;
    }

    pub async fn get_message(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<Message, sqlx::Error> {
        let message = sqlx::query!(
            r#"SELECT 
                messages.id, 
                messages.content, 
                messages.created_at,
                messages.attachments,
                users.name AS "author_username",
                users.avatar AS "author_avatar",
                users.id AS "author_id",
                members.nickname AS "author_nickname"
            FROM messages, members, users 
            WHERE (
                messages.id = $1 
                AND messages.channel_id = $2
                AND messages.guild_id = $3
                AND messages.user_id = users.id
                AND messages.user_id = members.user_id 
                AND messages.guild_id = members.guild_id
            )"#,
            message_id,
            channel_id,
            guild_id
        )
        .fetch_one(&self.pool)
        .map_ok(|record| {
            let attachments = match record.attachments.clone() {
                Some(some) => serde_json::from_value(some).ok(),
                None => None,
            };

            Message {
                id: record.id.clone(),
                content: record.content.clone(),
                attachments,
                created_at: DateTime::<Utc>::from_utc(record.created_at, Utc),
                author: PublicUserInfo {
                    id: record.author_id.clone(),
                    username: record.author_username.clone(),
                    avatar: record.author_avatar.clone(),
                },
            }
        })
        .await?;

        Ok(message)
    }

    #[allow(unused_variables)]
    pub async fn can_user_manage_messages(
        &self,
        user_id: &str,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        // TODO: implement, for now let's let people delete each other's
        // messages, freely, wild west style!
        return Ok(true);
    }
}
