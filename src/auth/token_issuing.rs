use crate::db::Database;

use chrono::{Duration, Utc};
use lazy_static::lazy_static;
use nanoid::nanoid;
use sqlx::{query, types::chrono::NaiveDateTime};

use crate::{
    auth::{access_token::AccessToken, token::Token, user::User},
    crypto,
};

lazy_static! {
    pub static ref REFRESH_TOKEN_VALIDITY: Duration = Duration::days(3);
    pub static ref ACCESS_TOKEN_VALIDITY: Duration = Duration::minutes(10);
}

pub enum IssueRefreshTokenResult {
    Failure,
    Success {
        user: User,
        access_token: AccessToken,
        refresh_token: Token,
    },
}

pub enum IssueAccessTokenResult {
    Failure,
    Success {
        access_token: AccessToken,
        refresh_token: Token,
    },
}

impl Database {
    async fn create_new_tokens_for(&self, user_id: &str, user_agent: &str) -> (AccessToken, Token) {
        let expires = Utc::now() + *REFRESH_TOKEN_VALIDITY;

        // add refresh token to db
        let nonce = query!(
            "INSERT INTO tokens (
                user_id, 
                token_id,
                nonce, 
                expires_at, 
                user_agent
            ) VALUES ($1, $2, $3, $4, $5) RETURNING nonce",
            user_id,
            nanoid!(),
            nanoid!(48),
            NaiveDateTime::from_timestamp_opt(expires.timestamp(), 0),
            user_agent
        )
        .fetch_one(&self.pool)
        .await
        .unwrap()
        .nonce;

        let refresh_token = Token::new(user_id.to_string(), expires, nonce);

        let access_token = AccessToken::new(user_id.to_string());

        return (access_token, refresh_token);
    }

    pub async fn issue_refresh_token(
        &self,
        email: &str,
        password: &str,
        user_agent: &str,
    ) -> IssueRefreshTokenResult {
        let user = query!(
            "SELECT id, name, email, avatar, password FROM users WHERE email = $1",
            email
        )
        .fetch_one(&self.pool)
        .await;

        match user {
            Ok(record) => {
                if !crypto::verify(password, &record.password) {
                    return IssueRefreshTokenResult::Failure;
                }

                let user = User {
                    id: record.id,
                    name: record.name,
                    email: record.email,
                    avatar: record.avatar,
                };

                let (access_token, refresh_token) =
                    self.create_new_tokens_for(&user.id, user_agent).await;

                return IssueRefreshTokenResult::Success {
                    user,
                    access_token,
                    refresh_token,
                };
            }
            Err(_) => {
                return IssueRefreshTokenResult::Failure;
            }
        }
    }

    pub async fn reissue_access_token(
        &self,
        refresh_token: Token,
        user_agent: &str,
    ) -> IssueAccessTokenResult {
        let rows_affected = query!(
            "DELETE FROM tokens WHERE user_id = $1 AND nonce = $2 AND expires_at > now()",
            refresh_token.user_id,
            refresh_token.proof
        )
        .execute(&self.pool)
        .await
        .unwrap()
        .rows_affected();

        if rows_affected == 0 {
            return IssueAccessTokenResult::Failure;
        }

        let (access_token, refresh_token) = self
            .create_new_tokens_for(&refresh_token.user_id, user_agent)
            .await;

        return IssueAccessTokenResult::Success {
            access_token,
            refresh_token,
        };
    }
}
