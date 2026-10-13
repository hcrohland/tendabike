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
    AxumAdmin, RequestSession,
    appstate::AppState,
    error::{ApiResult, AppError},
};
use tb_domain::{Activity, ActivityId, ApiWrite, ConcreteDomain, DescendReport, PartId, Store};
use tb_exec::{Txn, TxnSource};
use tb_strava::StravaStore;

async fn def_part_api<St, S>(
    user: RequestSession,
    State(state): State<AppState<St, S>>,
    Json(gear_id): Json<PartId>,
) -> Result<StatusCode, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ActivityDefaultGear { gear: gear_id },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn rescan<St, S>(
    _u: AxumAdmin,
    State(state): State<AppState<St, S>>,
) -> Result<StatusCode, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    Activity::rescan_all(&mut store).await?;
    store.commit().await?;
    // The rescan is all-users maintenance that bypasses the executors (the
    // spec §3/§6.4 carve-out), so no frame carries its effect: stop every
    // live executor — open streams end, and each client's native reconnect
    // + catch-up snapshot re-hydrates it (issue #446).
    state.registry.stop_all().await;
    Ok(StatusCode::NO_CONTENT)
}

/// web interface to read an activity
async fn act_get<St, S>(
    user: RequestSession,
    State(state): State<AppState<St, S>>,
    Path(id): Path<i64>,
) -> ApiResult<Activity>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    Ok(ActivityId::new(id)
        .read(&user, &mut store)
        .await
        .map(Json)?)
}

/// web interface to change an activity
async fn act_put<St, S>(
    Path(id): Path<i64>,
    user: RequestSession,
    State(state): State<AppState<St, S>>,
    Json(activity): Json<Activity>,
) -> Result<StatusCode, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    if ActivityId::from(id) != activity.id {
        Err(tb_domain::Error::BadRequest(
            "ActivityId does not match activity".to_string(),
        ))?
    }
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ActivityUpdate {
                id: ActivityId::from(id),
                activity,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// web interface to delete an activity
async fn act_delete<St, S>(
    Path(id): Path<i64>,
    user: RequestSession,
    State(state): State<AppState<St, S>>,
) -> Result<StatusCode, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ActivityDelete {
                id: ActivityId::new(id),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/activ/descend` — the raw CSV body. The write rides the
/// user's executor (like every write), and the handler answers `200` with
/// the match report `{good, bad}` (the spec §6.2 deviation recorded on
/// issue #446): the report is an operation result, not `Summary` state —
/// the matched activities' updated state arrives as the stream frame the
/// executor publishes.
async fn descend<St, S>(
    user: RequestSession,
    State(state): State<AppState<St, S>>,
    data: String,
) -> Result<Json<DescendReport>, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let report = state
        .registry
        .write_descend(&state.source, user.tb_id(), data)
        .await?;
    Ok(Json(report))
}

pub(crate) fn router<St, S>() -> Router<AppState<St, S>>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    Router::new()
        .route("/descend", post(descend))
        .route("/{id}", delete(act_delete).get(act_get).put(act_put))
        .route("/rescan", get(rescan))
        .route("/defaultgear", post(def_part_api))
}
