//! This module contains error handling utilities for the axum web framework.
//!
//! It provides functions for handling fallback errors, mapping any error into a `500 Internal Server Error`
//! response, and converting `anyhow::Error` into a custom `AppError` type.
//!
//! Additionally, it defines a custom `AppError` type that wraps `anyhow::Error` and implements the `IntoResponse`
//! trait for converting it into an HTTP response.
//!
//! Finally, it defines a type alias `ApiResult<T>` for `Result<Json<T>, AppError>`.
//!

use async_session::log::{error, info, debug};
use axum::{
    response::{IntoResponse, Response, Redirect},
    Json,
};
use http::StatusCode;
use tb_domain::Error;

pub async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        axum::response::Redirect::temporary("/strava/login").into_response()
    }
}

pub type ApiResult<T> = Result<Json<T>, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    TbError(#[from] Error),
    #[error(transparent)]
    AnyError(#[from] anyhow::Error),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = match &self {
            Self::TbError(err) => match err {
                Error::Forbidden(_) |
                Error::NotAuth(_) => return (StatusCode::UNAUTHORIZED, Redirect::temporary("/strava/logout")).into_response(),
                Error::NotFound(_) => StatusCode::NOT_FOUND,
                Error::BadRequest(_) => StatusCode::BAD_REQUEST,
                Error::Conflict(_) => StatusCode::CONFLICT,
                Error::TryAgain(_) => StatusCode::TOO_MANY_REQUESTS,
                Error::DatabaseFailure(_) |
                Error::AnyFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
            Self::AnyError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        let any: anyhow::Error = self.into();
        let msg = format!("{:#}", any);
        match code {
            StatusCode::INTERNAL_SERVER_ERROR =>  error!("{}", msg),
            StatusCode::BAD_REQUEST |
            StatusCode::NOT_FOUND => info!("{}", msg),
            _ => debug!("returning with error {}: {}", code.canonical_reason().unwrap_or(""), msg)
        };
        (code, msg).into_response()
    }
}
