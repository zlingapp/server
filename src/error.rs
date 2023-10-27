use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use derive_more::{Display, Error};
use log::error;
use serde::Serialize;

use crate::auth::token::TokenParseError;

pub type HResult<T> = std::result::Result<T, HandlerError>;

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

impl From<(u16, &'static str)> for HandlerError {
    fn from(tuple: (u16, &'static str)) -> Self {
        Self::with_code(tuple.0, tuple.1.into())
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


pub trait IntoHandlerErrorResult<T> {
    fn or_err(self, code: u16) -> Result<T, HandlerError>;
    fn or_err_msg(self, code: u16, message: &'static str) -> Result<T, HandlerError>;
}

impl<T, E> IntoHandlerErrorResult<T> for Result<T, E> {
    fn or_err(self, code: u16) -> Result<T, HandlerError> {
        Err(code.into())
    }

    fn or_err_msg(self, code: u16, message: &'static str) -> Result<T, HandlerError> {
        Err(HandlerError::from((code, message)))
    }
}

impl<T> IntoHandlerErrorResult<T> for Option<T> {
    fn or_err(self, code: u16) -> Result<T, HandlerError> {
        Err(code.into())
    }

    fn or_err_msg(self, code: u16, message: &'static str) -> Result<T, HandlerError> {
        Err(HandlerError::from((code, message)))
    }
}


pub mod macros {
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
