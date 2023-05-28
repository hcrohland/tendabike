//! This module contains types and functions related to handling activity and part types in the server.
//!
//! The `activity` and `part` functions are used to retrieve all activity and part types from the database.
//! The `router` function is used to create a router that handles requests related to activity and part types.

use axum::{Json, Router, routing::get};
use kernel::domain::{ActivityType, PartType};

use crate::{user::RUser, AppDbConn};

// get all activity types
async fn activity(_user: RUser, mut conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(ActivityType::all_ordered(&mut conn))
}

/// get all part types
async fn part(mut conn: AppDbConn) -> Json<Vec<PartType>> {
    Json(PartType::all_ordered(&mut conn))
}

pub(crate) fn router(state: crate::AppState) -> Router{
    Router::new()
        .route("/part", get(part))
        .route("/activity", get(activity))
        .with_state(state)
}
