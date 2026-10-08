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

use crate::{AxumAdmin, RequestSession, appstate::AppState, error::ApiResult};
use tb_domain::{Activity, ActivityId, PartId, Summary};
use tb_exec::{Txn, TxnSource};

async fn def_part_api<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(gear_id): Json<PartId>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let res = Activity::set_default_part(gear_id, &user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

async fn rescan<S>(_u: AxumAdmin, State(state): State<AppState<S>>) -> ApiResult<()>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    Activity::rescan_all(&mut store).await?;
    store.commit().await?;
    Ok(Json(()))
}

/// web interface to read an activity
async fn act_get<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Path(id): Path<i64>,
) -> ApiResult<Activity>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    Ok(ActivityId::new(id)
        .read(&user, &mut store)
        .await
        .map(Json)?)
}

/// web interface to change an activity
async fn act_put<S>(
    Path(id): Path<i64>,
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(activity): Json<Activity>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    if ActivityId::from(id) != activity.id {
        Err(tb_domain::Error::BadRequest(
            "ActivityId does not match activity".to_string(),
        ))?
    }
    let mut store = state.source.begin().await?;
    let res = activity.update(&user, &mut store).await.map(Json)?;
    store.commit().await?;
    Ok(res)
}

/// web interface to delete an activity
async fn act_delete<S>(
    Path(id): Path<i64>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let res = ActivityId::new(id)
        .delete(&user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

async fn descend<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    data: String,
) -> ApiResult<(Summary, Vec<String>, Vec<String>)>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let (summary, a, b) = Activity::csv2descend(data.as_bytes(), &user, &mut store).await?;
    store.commit().await?;
    Ok(Json((summary, a, b)))
}

pub(crate) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/descend", post(descend))
        .route("/{id}", delete(act_delete).get(act_get).put(act_put))
        .route("/rescan", get(rescan))
        .route("/defaultgear", post(def_part_api))
}
