//! This module contains the implementation of the attachment API endpoints.
//!
//! The attachment API allows users to attach and detach parts to other parts.
//! The API endpoints are `/attach` and `/detach`.
//!
//! The module defines two async functions `attach_rt` and `detach_rt` that handle the requests to the API endpoints.
//! The `router` function creates a new router and maps the API endpoints to their respective functions.

use axum::{routing::post, Json, Router, extract::State};
use tb_domain::{Event, Summary};

use crate::{error::ApiResult, user::RequestUser, DbPool, appstate::AppState};

/// route for attach API
async fn attach_rt(
    user: RequestUser,
    State(conn): State<DbPool>,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    Ok(event.attach(&user, &mut conn).await.map(Json)?)
}

/// route for detach API
async fn detach_rt(
    user: RequestUser,
    State(conn): State<DbPool>,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    Ok(event.detach(&user, &mut conn).await.map(Json)?)
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/attach", post(attach_rt))
        .route("/detach", post(detach_rt))
}
