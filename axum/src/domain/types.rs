//! This module contains types and functions related to handling activity and part types in the server.
//!
//! The `activity` and `part` functions are used to retrieve all activity and part types from the database.
//! The `router` function is used to create a router that handles requests related to activity and part types.

use axum::{extract::State, routing::get, Json, Router};

use crate::{appstate::AppState, error::ApiResult, DbPool};
use tb_domain::{ActivityType, PartType};

// get all activity types
async fn activity(State(store): State<DbPool>) -> ApiResult<Vec<ActivityType>> {
    let mut store = store.get().await?;
    Ok(Json(ActivityType::all_ordered(&mut store).await))
}

/// get all part types
async fn part(State(store): State<DbPool>) -> ApiResult<Vec<PartType>> {
    let mut store = store.get().await?;
    Ok(Json(PartType::all_ordered(&mut store).await))
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/part", get(part))
        .route("/activity", get(activity))
}
