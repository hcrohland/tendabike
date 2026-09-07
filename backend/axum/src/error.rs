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

use axum::{
    Json,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use log::{debug, error, info, warn};

use tb_domain::Error;

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
                Error::NotAuth(_) => StatusCode::UNAUTHORIZED,
                Error::Forbidden(_) => StatusCode::FORBIDDEN,
                Error::NotFound(_) => StatusCode::NOT_FOUND,
                Error::BadRequest(_) => StatusCode::BAD_REQUEST,
                Error::Conflict(_) => StatusCode::CONFLICT,
                Error::TryAgain(_) => StatusCode::TOO_MANY_REQUESTS,
                Error::DatabaseFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
                Error::AnyFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Self::AnyError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let any: anyhow::Error = self.into();
        let msg = format!("{any:#}");
        match code {
            StatusCode::INTERNAL_SERVER_ERROR => error!("{msg}"),
            StatusCode::BAD_REQUEST => warn!("{msg}"),
            StatusCode::NOT_FOUND => info!("{msg}"),
            _ => debug!(
                "returning with error {}: {msg}",
                code.canonical_reason().unwrap_or("")
            ),
        };
        (code, msg).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn respond(err: AppError) -> (StatusCode, String) {
        let res = err.into_response();
        let status = res.status();
        let body = to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("collect body");
        (status, std::str::from_utf8(&body).expect("utf8").to_owned())
    }

    #[tokio::test]
    async fn not_auth_maps_to_unauthorized() {
        let (status, body) = respond(Error::NotAuth("missing session".to_string()).into()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body, "User not authenticated: missing session");
    }

    #[tokio::test]
    async fn forbidden_maps_to_forbidden() {
        let (status, body) = respond(Error::Forbidden("not yours".to_string()).into()).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body, "Forbidden request: not yours");
    }

    #[tokio::test]
    async fn not_found_maps_to_not_found() {
        let (status, body) = respond(Error::NotFound("part 42".to_string()).into()).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, "Object not found: part 42");
    }

    #[tokio::test]
    async fn bad_request_maps_to_bad_request() {
        let (status, body) = respond(Error::BadRequest("malformed".to_string()).into()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body, "Bad Request: malformed");
    }

    #[tokio::test]
    async fn conflict_maps_to_conflict() {
        let (status, body) = respond(Error::Conflict("in use".to_string()).into()).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body, "Conflict: in use");
    }

    #[tokio::test]
    async fn try_again_maps_to_too_many_requests() {
        let (status, body) = respond(Error::TryAgain("rate limited").into()).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body, "Try again: rate limited");
    }

    #[tokio::test]
    async fn database_failure_maps_to_internal_server_error() {
        let (status, body) =
            respond(Error::DatabaseFailure(anyhow::anyhow!("db down")).into()).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "db down");
    }

    #[tokio::test]
    async fn any_failure_maps_to_internal_server_error() {
        let (status, body) = respond(Error::AnyFailure(anyhow::anyhow!("boom")).into()).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "boom");
    }

    #[tokio::test]
    async fn any_error_maps_to_internal_server_error() {
        let (status, body) = respond(AppError::AnyError(anyhow::anyhow!("any"))).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "any");
    }
}
