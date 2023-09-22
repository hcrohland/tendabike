//! This module contains types and functions related to handling activity and part types in the server.
//!
//! The `activity` and `part` functions are used to retrieve all activity and part types from the database.
//! The `router` function is used to create a router that handles requests related to activity and part types.

use axum::{Json, Router, routing::get, extract::State};
use tb_domain::{ActivityType, PartType};

use crate::{RequestUser, appstate::AppState, DbPool, error::ApiResult};

// get all activity types
async fn activity(_user: RequestUser, State(conn): State<DbPool>) -> ApiResult<Vec<ActivityType>> {
    let mut conn = conn.get().await?;
    Ok(Json(ActivityType::all_ordered(&mut conn).await))
}

/// get all part types
async fn part(State(conn): State<DbPool>) -> ApiResult<Vec<PartType>> {
    let mut conn = conn.get().await?;
    Ok(Json(PartType::all_ordered(&mut conn).await))
}

pub(crate) fn router() -> Router<AppState>{
    Router::new()
        .route("/part", get(part))
        .route("/activity", get(activity))
}
