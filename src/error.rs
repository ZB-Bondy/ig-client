/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 12/5/25
 ******************************************************************************/
use std::{fmt, io};
use std::fmt::{Display, Formatter};
use reqwest::StatusCode;

#[derive(Debug)]
pub enum FetchError {
    Reqwest(reqwest::Error),
    Sqlx(sqlx::Error),
    Parser(String),
}

impl Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Reqwest(e) => write!(f, "network error: {e}"),
            FetchError::Sqlx(e) => write!(f, "db error: {e}"),
            FetchError::Parser(msg) => write!(f, "parser error: {msg}"),
        }
    }
}

impl std::error::Error for FetchError {}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> Self {
        FetchError::Reqwest(err)
    }
}

impl From<sqlx::Error> for FetchError {
    fn from(err: sqlx::Error) -> Self {
        FetchError::Sqlx(err)
    }
}


#[derive(Debug)]
pub enum AuthError {
    Network(reqwest::Error),
    Io(io::Error),
    Json(serde_json::Error),
    Other(String),
    BadCredentials,
    Unexpected(StatusCode),
}

impl Display for AuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Network(e) => write!(f, "network error: {e}"),
            AuthError::Io(e)      => write!(f, "io error: {e}"),
            AuthError::Json(e)    => write!(f, "json error: {e}"),
            AuthError::Other(msg) => write!(f, "other error: {msg}"),
            AuthError::BadCredentials => write!(f, "bad credentials"),
            AuthError::Unexpected(s) => write!(f, "unexpected http status: {s}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<reqwest::Error> for AuthError {
    fn from(e: reqwest::Error) -> Self { AuthError::Network(e) }
}
impl From<Box<dyn std::error::Error + Send + Sync>> for AuthError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        match e.downcast::<reqwest::Error>() {
            Ok(req) => AuthError::Network(*req),
            Err(e) => match e.downcast::<serde_json::Error>() {
                Ok(js) => AuthError::Json(*js),
                Err(e) => match e.downcast::<std::io::Error>() {
                    Ok(ioe) => AuthError::Io(*ioe),
                    Err(other) => AuthError::Other(other.to_string()),
                },
            },
        }
    }
}
impl From<AppError> for AuthError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Network(e) => AuthError::Network(e),
            AppError::Io(e)      => AuthError::Io(e),
            AppError::Json(e)    => AuthError::Json(e),
            AppError::Unexpected(s) => AuthError::Unexpected(s),
            _ => AuthError::Other("unknown error".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    Network(reqwest::Error),
    Io(io::Error),
    Json(serde_json::Error),
    Unexpected(StatusCode),
    Db(sqlx::Error),
    Unauthorized,
    NotFound,
    RateLimitExceeded,
    SerializationError(String),
    WebSocketError(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Network(e)   => write!(f, "network error: {e}"),
            AppError::Io(e)        => write!(f, "io error: {e}"),
            AppError::Json(e)      => write!(f, "json error: {e}"),
            AppError::Unexpected(s)=> write!(f, "unexpected http status: {s}"),
            AppError::Db(e)        => write!(f, "db error: {e}"),
            AppError::Unauthorized  => write!(f, "unauthorized"),
            AppError::NotFound      => write!(f, "not found"),
            AppError::RateLimitExceeded => write!(f, "rate limit exceeded"),
            AppError::SerializationError(s) => write!(f, "serialization error: {s}"),
            AppError::WebSocketError(s) => write!(f, "websocket error: {s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self { AppError::Network(e) }
}
impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self { AppError::Io(e) }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self { AppError::Json(e) }
}
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}
impl From<AuthError> for AppError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::Network(e) => AppError::Network(e),
            AuthError::Io(e)      => AppError::Io(e),
            AuthError::Json(e)    => AppError::Json(e),
            AuthError::BadCredentials => AppError::Unauthorized,
            AuthError::Unexpected(s) => AppError::Unexpected(s),
            _ => AppError::Unexpected(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
