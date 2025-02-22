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
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use http::StatusCode;

use crate::{
    AxumAdmin, DbPool, RequestUser,
    appstate::AppState,
    error::{ApiResult, AppError},
};
use tb_domain::{Activity, ActivityId, NewActivity, PartId, PartTypeId, Summary};

async fn def_part_api(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(gear_id): Json<PartId>,
) -> ApiResult<Summary> {
    let mut store = store.get().await?;
    Ok(Activity::set_default_part(gear_id, &user, &mut store)
        .await
        .map(Json)?)
}

async fn rescan(_u: AxumAdmin, State(store): State<DbPool>) -> ApiResult<()> {
    let mut store = store.get().await?;
    Activity::rescan_all(&mut store).await?;
    Ok(Json(()))
}

/// web interface to read an activity
async fn act_get(
    user: RequestUser,
    State(store): State<DbPool>,
    Path(id): Path<i32>,
) -> ApiResult<Activity> {
    let mut store = store.get().await?;
    Ok(ActivityId::new(id)
        .read(&user, &mut store)
        .await
        .map(Json)?)
}

/// web interface to create an activity
async fn act_post(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(activity): Json<NewActivity>,
) -> Result<(StatusCode, Json<Summary>), AppError> {
    let mut store = store.get().await?;
    let assembly = Activity::create(&activity, &user, &mut store).await?;

    Ok((StatusCode::CREATED, Json(assembly)))
}

/// web interface to change an activity
async fn act_put(
    Path(id): Path<i32>,
    user: RequestUser,
    State(store): State<DbPool>,
    Json(activity): Json<NewActivity>,
) -> ApiResult<Summary> {
    let mut store = store.get().await?;
    Ok(ActivityId::new(id)
        .update(activity, &user, &mut store)
        .await
        .map(Json)?)
}

/// web interface to delete an activity
async fn act_delete(
    Path(id): Path<i32>,
    user: RequestUser,
    State(store): State<DbPool>,
) -> ApiResult<Summary> {
    let mut store = store.get().await?;
    Ok(ActivityId::new(id)
        .delete(&user, &mut store)
        .await
        .map(Json)?)
}

async fn descend(
    user: RequestUser,
    State(store): State<DbPool>,
    data: String,
) -> ApiResult<(Summary, Vec<String>, Vec<String>)> {
    let mut store = store.get().await?;
    let res = Activity::csv2descend(data.as_bytes(), &user, &mut store).await?;
    Ok(Json(res))
}

async fn mycats(user: RequestUser, State(store): State<DbPool>) -> ApiResult<HashSet<PartTypeId>> {
    let mut store = store.get().await?;
    Ok(Activity::categories(&user, &mut store).await.map(Json)?)
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(mycats))
        .route("/descend", post(descend))
        .route("/{id}", delete(act_delete).get(act_get).put(act_put))
        .route("/", post(act_post))
        .route("/rescan", get(rescan))
        .route("/defaultgear", post(def_part_api))
}
