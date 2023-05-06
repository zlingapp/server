use derive_more::{Display, Error};
use std::str::FromStr;
use time::OffsetDateTime;

use super::user::UserId;
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub user_id: UserId,
    pub expires: OffsetDateTime, // utc
    pub proof: String,
}

impl Token {
    pub fn new(user_id: String, expires: OffsetDateTime, proof: String) -> Self {
        Self {
            user_id,
            expires,
            proof,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires < OffsetDateTime::now_utc()
    }
}

impl ToString for Token {
    /// Serializes the token to a string.
    /// Signs the token using the `TOKEN_SIGNING_KEY`
    fn to_string(&self) -> String {
        // user id is not base64 encoded
        // let user_id = base64_url::encode(&self.user_id);
        let expiry: [u8; 4] = (self.expires.unix_timestamp() as u32).to_be_bytes();
        let expiry = base64_url::encode(&expiry);
        format!("{}.{}.{}", self.user_id, expiry, self.proof)
    }
}

#[derive(Debug, Display, Error)]
pub enum TokenParseError {
    InvalidFormat,
    Expired,
}

impl FromStr for Token {
    type Err = TokenParseError;

    /// Parses and validates a token from a string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let user_id = parts.next().ok_or(TokenParseError::InvalidFormat)?;
        let expires = parts.next().ok_or(TokenParseError::InvalidFormat)?;
        let proof = parts.next().ok_or(TokenParseError::InvalidFormat)?;

        if parts.next().is_some() {
            return Err(TokenParseError::InvalidFormat);
        }

        // user id is not base64 encoded
        // let user_id = base64_url::decode(user_id).map_err(|_| TokenParseError::InvalidFormat)?;
        let user_id =
            String::from_utf8(user_id.into()).map_err(|_| TokenParseError::InvalidFormat)?;

        let expires = base64_url::decode(expires).map_err(|_| TokenParseError::InvalidFormat)?;
        let expires: [u8; 4] = expires
            .as_slice()
            .try_into()
            .map_err(|_| TokenParseError::InvalidFormat)?;
        let expires = u32::from_be_bytes(expires);

        let expires = OffsetDateTime::from_unix_timestamp(expires as i64)
            .map_err(|_| TokenParseError::InvalidFormat)?;

        let tok = Self::new(user_id, expires, proof.to_string());
        if tok.is_expired() {
            return Err(TokenParseError::Expired);
        }

        Ok(tok)
    }
}
