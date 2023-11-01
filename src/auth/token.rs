use chrono::{DateTime, NaiveDateTime, Utc};
use derive_more::{Display, Error};
use serde::Deserializer;
use serde_json::json;
use std::{fmt::Display, str::FromStr};
use utoipa::{
    openapi::{Object, RefOr, Schema, SchemaType},
    ToSchema,
};

#[derive(Debug, PartialEq, Eq, Clone)]
// see impl of ToSchema below
pub struct Token {
    pub user_id: String,
    pub expires: DateTime<Utc>, // utc
    pub proof: String,
}

impl Token {
    pub fn new(user_id: String, expires: DateTime<Utc>, proof: String) -> Self {
        Self {
            user_id,
            expires,
            proof,
        }
    }

    pub fn is_expired(&self) -> bool {
        // check if expiry is before now
        self.expires < Utc::now()
    }

    pub fn is_bot(&self) -> bool {
        self.user_id.starts_with("bot:")
    }
}

impl Display for Token {
    /// Serializes the token to a string.
    /// Signs the token using the `TOKEN_SIGNING_KEY`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expiry: [u8; 4] = (self.expires.timestamp() as u32).to_be_bytes();
        let expiry = base64_url::encode(&expiry);

        f.write_fmt(format_args!("{}.{}.{}", self.user_id, expiry, self.proof))
    }
}

/// OpenAPI schema that represents the token as a string
impl ToSchema<'_> for Token {
    fn schema() -> (
        &'static str,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    ) {
        let mut obj = Object::with_type(SchemaType::String);
        obj.example = Some(json!(
            "xoKM4W7NDqHjK_V0g9s3y.ZN7jGQ.iIuDsgiT4s2ehQ-3ATImimyPUoooTPC1ytqqQuPQSJU"
        ));

        ("Token", RefOr::T(Schema::Object(obj)))
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

        let expires = NaiveDateTime::from_timestamp_opt(expires as i64, 0)
            .ok_or(TokenParseError::InvalidFormat)?
            .and_local_timezone(Utc)
            .single()
            .ok_or(TokenParseError::InvalidFormat)?;

        let tok = Self::new(user_id, expires, proof.to_string());
        if tok.is_expired() {
            return Err(TokenParseError::Expired);
        }

        Ok(tok)
    }
}

impl<'de> serde::Deserialize<'de> for Token {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// serialize with use_display
impl serde::Serialize for Token {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        crate::util::use_display(self, s)
    }
}
