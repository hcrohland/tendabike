use std::collections::HashSet;

/// This module contains the web interface for managing activities.
///
/// Activities are a central concept in the Tendabike application. They represent
/// a user's cycling activity, and can be created, read, updated, and deleted
/// through the web interface provided by this module.
///
/// The module also provides endpoints for managing activity parts, such as
/// setting a default part and rescanning all parts.
///
/// Finally, the module provides an endpoint for using CSV data to update usage data for activities.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use http::StatusCode;
use tb_domain::{Activity, ActivityId, NewActivity, PartId, PartTypeId, Summary};

use crate::{
    error::{ApiResult, AppError},
    AxumAdmin, RequestUser,
    DbPool, appstate::AppState,
};

async fn def_part_api(
    user: RequestUser,
    State(conn): State<DbPool>,
    Json(gear_id): Json<PartId>,
) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    Ok(Activity::set_default_part(gear_id, &user, &mut conn).await.map(Json)?)
}

async fn rescan(_u: AxumAdmin, State(conn): State<DbPool>) -> Result<(), AppError> {
    let mut conn = conn.get().await?;
    Activity::rescan_all(&mut conn).await?;
    Ok(())
}

/// web interface to read an activity
async fn act_get(user: RequestUser, State(conn): State<DbPool>, Path(id): Path<i32>) -> ApiResult<Activity> {
    let mut conn = conn.get().await?;
    Ok(ActivityId::new(id).read(&user, &mut conn).await.map(Json)?)
}

/// web interface to create an activity
async fn act_post(
    user: RequestUser,
    State(conn): State<DbPool>,
    Json(activity): Json<NewActivity>,
) -> Result<(StatusCode, Json<Summary>), AppError> {
    let mut conn = conn.get().await?;
    let assembly = Activity::create(&activity, &user, &mut conn).await?;

    Ok((StatusCode::CREATED, Json(assembly)))
}

/// web interface to change an activity
async fn act_put(
    Path(id): Path<i32>,
    user: RequestUser,
    State(conn): State<DbPool>,
    activity: Json<NewActivity>,
) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    Ok(ActivityId::new(id)
        .update(&activity, &user, &mut conn).await
        .map(Json)?)
}

/// web interface to delete an activity
async fn act_delete(Path(id): Path<i32>, user: RequestUser, State(conn): State<DbPool>) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    Ok(ActivityId::new(id).delete(&user, &mut conn).await.map(Json)?)
}

#[derive(serde::Deserialize)]
struct QueryTZ {
    tz: String,
}
async fn descend(Query(q): Query<QueryTZ>, user: RequestUser, State(conn): State<DbPool>, data: String) -> ApiResult<(Summary, Vec<String>, Vec<String>)> {
    let mut conn = conn.get().await?;
    let res = Activity::csv2descend(data.as_bytes(), q.tz, &user, &mut conn).await?;
    Ok(Json(res))
}

async fn mycats(user: RequestUser, State(conn): State<DbPool>) -> ApiResult<HashSet<PartTypeId>> {
    let mut conn = conn.get().await?;
    Ok(Activity::categories(&user, &mut conn).await.map(Json)?)
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(mycats))
        .route("/descend", post(descend))
        .route("/:id", delete(act_delete).get(act_get).put(act_put))
        .route("/", post(act_post))
        .route("/rescan", get(rescan))
        .route("/defaultgear", post(def_part_api))
}
