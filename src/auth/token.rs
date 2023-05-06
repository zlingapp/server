use std::str::FromStr;
use std::{ops::Deref, pin::Pin};

use actix_web::FromRequest;
use derive_more::{Display, Error};
use futures::Future;
use lazy_static::lazy_static;
use time::{Duration, OffsetDateTime};

use crate::{crypto, options::TOKEN_SIGNING_KEY};

use super::user::UserId;

lazy_static! {
    pub static ref TOKEN_VALIDITY: Duration = Duration::hours(1);
}

/// Example token:
/// ```
///    xoKM4W7NDqHjK_V0g9s3y.ZFZDYw.iIuDsgiT4s2ehQ-3ATImimyPUoooTPC1ytqqQuPQSJU
///
///    AAAAAAAAAAAAAAAAAAAAA.BBBBBB.CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
///    ~~~~~~~~~~~~~~~~~~~~~ ~~~~~~ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
///            user_id       expiry                 signature
/// ```
///
/// Where `expiry` = `BASE64URL(unix_timestamp.big_endian_bytes)`
/// and `signature` = `BASE64URL(HMACSHA256_SIGN(user_id + "." + BASE64URL(expiry), TOKEN_SIGNING_KEY))`
///
/// Note: `user_id` is not base64 encoded
/// Note: `expiry` is a unix timestamp encoded as a base64url string, bytes are encoded in big endian (network order)
///
/// In the example:
/// - `user_id` = `xoKM4W7NDqHjK_V0g9s3y`
/// - `expiry` = `BASE64URL_DECODE("ZFZDYw") = 0x64564363 = 1683374947 (big-endian) = Sat May 06 2023 12:09:07 GMT+0000`
///
/// Below, the ToString and FromStr implementations for Token are provided.
/// ToString performs signing and serializes the token to a string.
/// FromStr parses a string and verifies the signature.
///
/// # Examples
///
/// You can try serializing a token to a string and parsing it back to a token using the following code:
/// ```
/// let token = Token::new(nanoid!());
/// let token_str = token.to_string();
/// let parsed_token = Token::from_str(&token_str).unwrap();
/// assert_eq!(token, parsed_token);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub user_id: UserId,
    pub expires: OffsetDateTime, // utc
}

impl Token {
    pub fn with_expiry(user_id: String, expires: OffsetDateTime) -> Self {
        Self { user_id, expires }
    }

    pub fn new(user_id: String) -> Self {
        Self::with_expiry(user_id, OffsetDateTime::now_utc() + *TOKEN_VALIDITY)
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

        let payload = format!("{}.{}", self.user_id, expiry);

        let signature = crypto::sign(&*TOKEN_SIGNING_KEY, payload.as_bytes());
        let signature = base64_url::encode(&signature);

        format!("{}.{}", payload, signature)
    }
}

#[derive(Debug, Display, Error)]
pub enum TokenParseError {
    InvalidFormat,
    InvalidSignature,
    Expired,
}

impl FromStr for Token {
    type Err = TokenParseError;
    
    /// Parses and validates a token from a string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let user_id = parts.next().ok_or(TokenParseError::InvalidFormat)?;
        let expires = parts.next().ok_or(TokenParseError::InvalidFormat)?;
        let signature = parts.next().ok_or(TokenParseError::InvalidFormat)?;

        if parts.next().is_some() {
            return Err(TokenParseError::InvalidFormat);
        }

        let signature =
            base64_url::decode(signature).map_err(|_| TokenParseError::InvalidFormat)?;

        let payload = format!("{}.{}", user_id, expires);
        if !crypto::verify_signature(&*TOKEN_SIGNING_KEY, payload.as_bytes(), &signature) {
            return Err(TokenParseError::InvalidSignature);
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

        let tok = Self::with_expiry(user_id, expires);
        if tok.is_expired() {
            return Err(TokenParseError::Expired);
        }

        Ok(tok)
    }
}

pub struct TokenEx(Token);

impl Deref for TokenEx {
    type Target = Token;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for TokenEx {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            use actix_web::error::ErrorUnauthorized;

            // get the authorization header
            let auth_header = req
                .headers()
                .get("Authorization")
                .map(|v| v.to_str())
                .ok_or(ErrorUnauthorized("access_denied"))?
                .map_err(|_| ErrorUnauthorized("access_denied"))?;

            // needs to be a Bearer token
            let token = auth_header.strip_prefix("Bearer ").ok_or(ErrorUnauthorized("access_denied"))?;

            // parse & validate the token
            let token = Token::from_str(token).map_err(|_| ErrorUnauthorized("access_denied"))?;

            Ok(TokenEx(token))
        })
    }
}
