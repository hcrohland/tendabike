//! This module contains the implementation of the attachment API endpoints.
//!
//! The attachment API allows users to attach and detach parts to other parts.
//! The API endpoints are `/attach` and `/detach`.
//!
//! The module defines two async functions `attach_rt` and `detach_rt` that handle the requests to the API endpoints.
//! The `router` function creates a new router and maps the API endpoints to their respective functions.

use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::log::debug;

use crate::{DbPool, RequestUser, appstate::AppState, error::ApiResult};
use tb_domain::{PartId, PartTypeId, Summary};

/// Description of an Attach or Detach request

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Event {
    /// the part which should be change
    part_id: PartId,
    /// when it the change happens
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    /// The gear the part is or will be attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
    /// if true, the the whole assembly will be detached
    all: bool,
}

/// route for attach API
async fn attach_rt(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    let mut store = store.get().await?;
    debug!("attach {event:?}");

    let Event {
        part_id,
        time,
        gear,
        hook,
        all,
    } = event;

    Ok(
        tb_domain::attach_assembly(&user, part_id, time, gear, hook, all, &mut store)
            .await
            .map(Json)?,
    )
}

/// route for detach API
async fn detach_rt(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    let mut store = store.get().await?;
    debug!("detach {event:?}");
    let Event {
        part_id, time, all, ..
    } = event;
    Ok(
        tb_domain::detach_assembly(&user, part_id, time, all, &mut store)
            .await
            .map(Json)?,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Dispose {
    part: PartId,
    time: OffsetDateTime,
    all: bool,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/attach", post(attach_rt))
        .route("/detach", post(detach_rt))
}
