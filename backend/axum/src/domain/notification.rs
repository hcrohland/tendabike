//! This module contains the implementation of the notification API endpoints.
//!
//! The endpoints support:
//!
//! - `POST /{id}/read` — read a single notification (lifecycle-aware:
//!   persistent → mark seen, transient → delete)
//! - `POST /read-all` — read all of the user's notifications
//! - `DELETE /{id}` — delete a notification (any of the recipient's)
//!
//! The handlers are transport-only: each extracts the path and session,
//! enqueues the `ApiWrite` on the user's executor, and answers `204`.
//! The recipient guard lives in the domain layer (ADR-0008).

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, post},
};
use tb_domain::{ApiWrite, NotificationId};

use crate::{RequestSession, appstate::AppState, error::AppError};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/read", post(read_notification))
        .route("/read-all", post(read_all_notifications))
        .route("/{id}", delete(delete_notification))
}

async fn read_notification(
    Path(id): Path<NotificationId>,
    user: RequestSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    state
        .api
        .write(Box::new(user), ApiWrite::NotificationRead { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn read_all_notifications(
    user: RequestSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    state
        .api
        .write(Box::new(user), ApiWrite::NotificationReadAll)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_notification(
    Path(id): Path<NotificationId>,
    user: RequestSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    state
        .api
        .write(Box::new(user), ApiWrite::NotificationDelete { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
