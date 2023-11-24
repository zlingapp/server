use actix_web::web::Data;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use nanoid::nanoid;
use sqlx::query;
use sqlx::{Pool, Postgres};

use crate::auth::user::{PublicUserInfo, User};
use crate::crypto;
use crate::friends::friend_request::{FriendRequest, FriendRequestType};
use crate::messaging::message::Message;

pub type DB = Data<Database>;

pub struct Database {
    pub pool: Pool<Postgres>,
}

pub struct Friends {
    pub friends: Vec<String>,
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
                        members.guild_id = channels.guild_id
                    )
            ) AS can_user_see_channel
            ",
            channel_id,
            user_id
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
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        self.can_user_see_channel(user_id, channel_id).await
    }

    pub async fn can_user_read_message_history_from(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        self.can_user_see_channel(user_id, channel_id).await
        // TODO cumulatively check all parent channels
    }

    /// this is different to message history! this is the ability to read messages
    /// even if they have been sent after the user has joined the channel.
    /// if this is false, it takes priority over "read message history".
    pub async fn can_user_view_messages_in(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        self.can_user_see_channel(user_id, channel_id).await
    }

    // TODO: Add permissions
    pub async fn can_user_create_invite_in(
        &self,
        user_id: &str,
        guild_id: &str,
    ) -> Result<bool, sqlx::Error> {
        self.is_user_in_guild(user_id, guild_id).await
    }

    pub async fn can_user_delete_invite(
        &self,
        user_id: &str,
        invite_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let owner = sqlx::query!(
            r#"SELECT guilds.owner 
                FROM guilds,invites 
                WHERE invites.code = $1 
                AND guilds.id=invites.guild_id"#,
            invite_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(owner.owner == user_id)
    }

    pub async fn get_message(
        &self,
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
                AND users.id = messages.user_id
                AND members.user_id = messages.user_id
            )"#,
            message_id,
            channel_id,
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
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(record.created_at, Utc),
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
        channel_id: &str,
    ) -> Result<bool, sqlx::Error> {
        // TODO: implement, for now let's let people delete each other's
        // messages, freely, wild west style!
        Ok(true)
    }

    pub async fn is_user_friend(&self, me_id: &str, friend_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_as!(
            Friends,
            r#"SELECT friends 
                FROM users 
                WHERE id = $1"#,
            me_id
        )
        .fetch_one(&self.pool)
        .await
        .map(|e| e.friends.contains(&friend_id.to_string()))
    }

    pub async fn list_incoming_friend_requests(
        &self,
        id: &str,
    ) -> Result<Vec<FriendRequest>, sqlx::Error> {
        sqlx::query_as!(
            PublicUserInfo,
            r#"SELECT id, name AS "username", avatar 
               FROM friend_requests, users
               WHERE from_user = id AND to_user = $1"#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .map(|r| {
            r.into_iter()
                .map(|i| FriendRequest {
                    direction: FriendRequestType::Incoming,
                    user: i,
                })
                .collect::<Vec<FriendRequest>>()
        })
    }
    pub async fn list_outgoing_friend_requests(
        &self,
        id: &str,
    ) -> Result<Vec<FriendRequest>, sqlx::Error> {
        sqlx::query_as!(
            PublicUserInfo,
            r#"SELECT id,name as "username",avatar 
                FROM friend_requests, users
                WHERE to_user = id AND from_user = $1"#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .map(|r| {
            r.into_iter()
                .map(|i| FriendRequest {
                    direction: FriendRequestType::Outgoing,
                    user: i,
                })
                .collect::<Vec<FriendRequest>>()
        })
    }
    pub async fn add_friends(&self, id1: &str, id2: &str) -> Result<(), sqlx::Error> {
        // This can create duplicate friends, be careful
        sqlx::query!(
            r#"UPDATE users
                SET friends = ARRAY_APPEND(friends,$1)
                WHERE id=$2"#,
            id1,
            id2
        )
        .execute(&self.pool)
        .await?;
        sqlx::query!(
            r#"UPDATE users
                SET friends = ARRAY_APPEND(friends,$1)
                WHERE id=$2"#,
            id2,
            id1
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn get_dm_channel_id(
        &self,
        from_id: &str,
        to_id: &str,
    ) -> Result<String, sqlx::Error> {
        let user1 = std::cmp::min(from_id, to_id);
        let user2 = std::cmp::max(from_id, to_id);

        let existing_channel = sqlx::query!(
            "SELECT id FROM dmchannels WHERE from_user = $1 AND to_user = $2",
            user1,
            user2
        )
        .fetch_optional(&self.pool)
        .await?;

        let channel_id = match existing_channel {
            // return the existing channel's id
            Some(channel) => channel.id,
            // create a new channel
            None => {
                sqlx::query!(
                "INSERT INTO dmchannels (id, from_user, to_user) VALUES ($1, $2, $3) RETURNING id",
                nanoid!(),
                user1,
                user2
            )
                .fetch_one(&self.pool)
                .await?
                .id
            }
        };

        Ok(channel_id)
    }
}
