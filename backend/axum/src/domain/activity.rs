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
    domain::result_mismatch,
    error::{ApiResult, AppError},
};
use tb_domain::{
    Activity, ActivityId, ApiRead, ApiWrite, DescendReport, PartId, ReadOutcome, Session,
    WriteOutcome,
};

async fn def_part_api(
    user: RequestSession,
    State(state): State<AppState>,
    Json(gear_id): Json<PartId>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            user.user_id(),
            user.shop(),
            ApiWrite::ActivityDefaultGear { gear: gear_id },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn rescan(_u: AxumAdmin, State(state): State<AppState>) -> Result<StatusCode, AppError> {
    // The rescan is all-users maintenance that bypasses the executors (the
    // spec §3/§6.4 carve-out): the service API's direct op runs the one
    // transaction (`Activity::rescan_all`), then stops every live executor
    // — open streams end, and each client's native reconnect + catch-up
    // snapshot re-hydrates it (issue #446).
    state.api.rescan().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// web interface to read an activity
async fn act_get(
    user: RequestSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Activity> {
    match state
        .api
        .read(user.user_id(), ApiRead::Activity(ActivityId::new(id)))
        .await?
    {
        ReadOutcome::Activity(activity) => Ok(Json(activity)),
        _ => Err(result_mismatch("activity", "activity")),
    }
}

/// web interface to change an activity
async fn act_put(
    Path(id): Path<i64>,
    user: RequestSession,
    State(state): State<AppState>,
    Json(activity): Json<Activity>,
) -> Result<StatusCode, AppError> {
    if ActivityId::from(id) != activity.id {
        Err(tb_domain::Error::BadRequest(
            "ActivityId does not match activity".to_string(),
        ))?
    }
    state
        .api
        .write(
            user.user_id(),
            user.shop(),
            ApiWrite::ActivityUpdate {
                id: ActivityId::from(id),
                activity,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// web interface to delete an activity
async fn act_delete(
    Path(id): Path<i64>,
    user: RequestSession,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            user.user_id(),
            user.shop(),
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
async fn descend(
    user: RequestSession,
    State(state): State<AppState>,
    data: String,
) -> Result<Json<DescendReport>, AppError> {
    // The write's outcome carries the report (the response body, not state
    // — it never rides the frame): the handler knows it sent the descend,
    // any other outcome is a bug, guarded by the outcome mismatch.
    let report = match state
        .api
        .write(
            user.user_id(),
            user.shop(),
            ApiWrite::ActivityDescend { data },
        )
        .await?
    {
        WriteOutcome::Descend { report, .. } => report,
        _ => return Err(result_mismatch("descend", "report")),
    };
    Ok(Json(report))
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/descend", post(descend))
        .route("/{id}", delete(act_delete).get(act_get).put(act_put))
        .route("/rescan", get(rescan))
        .route("/defaultgear", post(def_part_api))
}
