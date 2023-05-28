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

use axum::{response::{IntoResponse, Response}, Json};
use http::StatusCode;

pub async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub fn internal_any(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        axum::response::Redirect::temporary("/strava/login").into_response()
    }
}

pub type ApiResult<T> = Result<Json<T>, AppError>;

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        async_session::log::error!("Internal server error: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
        .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
