//! This module contains the implementation of the attachment API endpoints.
//!
//! The attachment API allows users to attach and detach parts to other parts.
//! The API endpoints are `/attach` and `/detach`.
//!
//! The module defines two async functions `attach_rt` and `detach_rt` that handle the requests to the API endpoints.
//! The `router` function creates a new router and maps the API endpoints to their respective functions.

use axum::{routing::post, Json, Router};
use kernel::domain::{Event, Summary};

use crate::{error::ApiResult, user::RUser, AppDbConn};

/// route for attach API
async fn attach_rt(
    user: RUser,
    mut conn: AppDbConn,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    Ok(event.attach(&user, &mut conn).map(Json)?)
}

/// route for detach API
async fn detach_rt(
    user: RUser,
    mut conn: AppDbConn,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    Ok(event.detach(&user, &mut conn).map(Json)?)
}

pub(crate) fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/attach", post(attach_rt))
        .route("/detach", post(detach_rt))
        .with_state(state)
}
