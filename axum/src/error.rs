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
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;

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

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    TbError(#[from] tb_domain::Error),
    #[error(transparent)]
    AnyError(#[from] anyhow::Error),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = match &self {
            Self::TbError(err) => match err {
                tb_domain::Error::Forbidden(_) |
                tb_domain::Error::NotAuth(_) => StatusCode::FORBIDDEN,
                tb_domain::Error::NotFound(_) => StatusCode::NOT_FOUND,
                tb_domain::Error::BadRequest(_) => StatusCode::BAD_REQUEST,
                tb_domain::Error::Conflict(_) => StatusCode::CONFLICT,
                tb_domain::Error::TryAgain(_) => StatusCode::TOO_MANY_REQUESTS,
                tb_domain::Error::DatabaseFailure(_) |
                tb_domain::Error::AnyFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
            Self::AnyError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        match code {
            StatusCode::INTERNAL_SERVER_ERROR =>  error!("Internal server error : {}", self),
            StatusCode::BAD_REQUEST => info!("bad request: {}", self),
            StatusCode::NOT_FOUND => info!("not found: {}", self),
            _ => debug!("returning with error: {}", self)
        };
        (code, self.to_string()).into_response()
    }
}
