//! This module contains types and functions related to handling activity and part types in the server.
//!
//! The `activity` and `part` functions are used to retrieve all activity and part types from the database.
//! The `router` function is used to create a router that handles requests related to activity and part types.

use axum::{routing::get, Json, Router};

use crate::{appstate::AppState, error::ApiResult};
use tb_domain::{ActivityType, PartType};

// get all activity types
async fn activity() -> ApiResult<Vec<ActivityType>> {
    Ok(Json(ActivityType::all_ordered()))
}

/// get all part types
async fn part() -> ApiResult<Vec<PartType>> {
    Ok(Json(PartType::all_ordered()))
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/part", get(part))
        .route("/activity", get(activity))
}
