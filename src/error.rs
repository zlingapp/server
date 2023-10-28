use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use derive_more::{Display, Error};
use log::error;
use serde::Serialize;

use crate::auth::token::TokenParseError;

pub type HResult<T> = std::result::Result<T, HandlerError>;

/// A custom error type that should be used for all errors that occur in the
/// route handler functions. This type implements `ResponseError` and can be
/// returned from a handler function to send an error response to the client.
///
/// Conveniently, this type implements `From` for `sqlx::Error` and `u16` to
/// make it easy to return an error from a database query or a status code.
///
/// It is recommended that you use the `err!` macro to return an error from a
/// handler function. This macro is re-exported from this module.
#[derive(Debug, Display, Error, Serialize)]
#[display(fmt = "{}", message)]
pub struct HandlerError {
    pub message: String,
    pub code: u16,
}

impl HandlerError {
    pub fn with_code(code: u16, message: String) -> Self {
        Self { message, code }
    }

    pub fn internal_error() -> Self {
        Self::with_code(500, "Internal Server Error".into())
    }
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self)
    }
}

impl From<sqlx::Error> for HandlerError {
    fn from(err: sqlx::Error) -> Self {
        error!("database error: {}", err);
        Self::internal_error()
    }
}

impl From<u16> for HandlerError {
    fn from(code: u16) -> Self {
        let message = match code {
            403 => "Access denied".into(),
            401 => "Authorization required".into(),
            _ => StatusCode::from_u16(code)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                .to_string(),
            // Serialises according to StatusCode.to_string()
            // 500 -> "500" Internal Server Error
            // 409 -> "409 Conflict"
            // 404 -> "404 Not Found"
            // 403 -> "403 Forbidden"
            // 401 -> "401 Unauthorized"
        };

        Self::with_code(code, message)
    }
}

impl From<&str> for HandlerError {
    fn from(message: &str) -> Self {
        Self::with_code(500, message.into())
    }
}

impl From<String> for HandlerError {
    fn from(message: String) -> Self {
        Self::with_code(500, message)
    }
}

impl From<(u16, String)> for HandlerError {
    fn from(tuple: (u16, String)) -> Self {
        Self::with_code(tuple.0, tuple.1)
    }
}

impl From<TokenParseError> for HandlerError {
    fn from(err: TokenParseError) -> Self {
        use TokenParseError::*;
        match err {
            InvalidFormat => Self::with_code(403, "Invalid token supplied.".into()),
            Expired => Self::with_code(403, "The supplied token has expired.".into()),
        }
    }
}

/// Trait to facilitate the conversion between `Result` and `Option` into a
/// `HandlerError`. Useful when you have a `Result` or `Option` and want to
/// return an error if it is `None` or `Err`.
///
/// Example:
/// ```
///
/// fn handler(number: &str, double: &str) -> HResult<i32> {
///     //                                    ~~~~~~~~~~~~ use when possible
///     // Returns HTTP 400 Bad Request
///     let mut number: i32 = number.parse().or_err(400)?;
///     // Returns HTTP 400 Bad Request with a custom message
///     let double: bool = double.parse().or_err_msg(400, "must be true or false")?;
///
///     if double {
///         number *= 2;
///     }
///
///     Ok(number)
/// }
/// ```
pub trait IntoHandlerErrorResult<T> {
    fn or_err(self, code: u16) -> Result<T, HandlerError>;
    fn or_err_msg(self, code: u16, message: &'static str) -> Result<T, HandlerError>;
}

impl<T, E> IntoHandlerErrorResult<T> for Result<T, E> {
    fn or_err(self, code: u16) -> Result<T, HandlerError> {
        self.or(Err(code.into()))
    }

    fn or_err_msg(self, code: u16, message: &'static str) -> Result<T, HandlerError> {
        self.or(Err(HandlerError::from((code, message.into()))))
    }
}

impl<T> IntoHandlerErrorResult<T> for Option<T> {
    fn or_err(self, code: u16) -> Result<T, HandlerError> {
        self.ok_or(code.into())
    }

    fn or_err_msg(self, code: u16, message: &'static str) -> Result<T, HandlerError> {
        self.ok_or(HandlerError::from((code, message.into())))
    }
}

pub mod macros {
    /// Convenience macro to return a `HandlerError`` wrapped in an `Err()``.
    /// Use it whenever you want to return an error from a handler function.
    ///
    /// Example:
    /// ```
    /// use crate::error::macros::err;
    ///
    /// fn handler() -> HResult<()> {
    ///    // Returns HTTP 500 Internal Server Error
    ///    err!()?;
    ///    
    ///    // Returns HTTP 403 Forbidden
    ///    err!(403)?;
    ///    
    ///    // Returns HTTP 403 Forbidden with a custom message
    ///    err!(403, "You shall not pass!")?;
    ///    
    ///    // Returns HTTP 500 Internal Server Error with a custom message
    ///    err!("A rat chewed through the ethernet cable")?;
    ///    
    ///    Ok(())
    /// }
    macro_rules! err {
        ($code:expr, $msg:expr) => {
            Err(crate::error::HandlerError::from(($code, $msg.into())))
        };
        ($code:expr) => {
            Err(crate::error::HandlerError::from($code))
        };
        () => {
            Err(crate::error::HandlerError::internal_error())
        };
    }

    pub(crate) use err;
}
