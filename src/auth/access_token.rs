use std::str::FromStr;
use std::{ops::Deref, pin::Pin};

use actix_web::FromRequest;
use chrono::{Utc, DateTime};
use derive_more::{Display, Error};
use futures::Future;

use crate::{crypto, options::TOKEN_SIGNING_KEY};

use super::token::{Token, TokenParseError};
use super::token_issuing::ACCESS_TOKEN_VALIDITY;

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
/// let token = AccessToken::new(nanoid!());
/// let token_str = AccessToken.to_string();
/// let parsed_token = Token::from_str(&token_str).unwrap();
/// assert_eq!(token, parsed_token);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct AccessToken(Token);

impl AccessToken {
    pub fn from_existing(token: Token) -> Option<Self> {
        let access_token = AccessToken(token);

        if access_token.is_signature_valid() {
            Some(access_token)
        } else {
            None
        }
    }

    pub fn new(user_id: String) -> Self {
        let expires = Utc::now() + *ACCESS_TOKEN_VALIDITY;
        Self::with_expiry(user_id, expires)
    }

    pub fn with_expiry(user_id: String, expires: DateTime<Utc>) -> Self {
        let mut token = Token::new(user_id, expires, "".to_string());

        let serialized = token.to_string();
        let payload = serialized.strip_suffix(".").unwrap().as_bytes();

        // signs the payload and appends the signature
        token.proof = base64_url::encode(&crypto::sign(&*TOKEN_SIGNING_KEY, payload));
        AccessToken(token)
    }

    pub fn is_signature_valid(&self) -> bool {
        let signature = match base64_url::decode(&self.proof) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let token = Token::new(self.user_id.clone(), self.expires, "".to_string());
        let serialized = token.to_string();
        let payload = serialized.strip_suffix(".").unwrap().as_bytes();

        crypto::verify_signature(&*TOKEN_SIGNING_KEY, payload, &signature)
    }
}

#[derive(Debug, Display, Error)]
pub enum AccessTokenParseError {
    SignatureInvalid,
    TokenInvalid(TokenParseError),
}

impl FromStr for AccessToken {
    type Err = AccessTokenParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let token = Token::from_str(s).map_err(|e| AccessTokenParseError::TokenInvalid(e))?;
        Ok(AccessToken::from_existing(token).ok_or(AccessTokenParseError::SignatureInvalid)?)
    }
}

impl Deref for AccessToken {
    type Target = Token;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for AccessToken {
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
                .ok_or(ErrorUnauthorized("authentication_required"))?
                .map_err(|_| ErrorUnauthorized("authentication_required"))?;

            // needs to be a Bearer token
            let token = auth_header
                .strip_prefix("Bearer ")
                .ok_or(ErrorUnauthorized("authentication_required"))?;

            // parse & validate the token
            let access_token: AccessToken = token
                .parse()
                .map_err(|_| ErrorUnauthorized("authentication_required"))?;

            Ok(access_token)
        })
    }
}
