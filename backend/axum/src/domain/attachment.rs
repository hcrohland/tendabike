//! This module contains the implementation of the attachment API endpoints.
//!
//! The attachment API allows users to attach and detach parts to other parts.
//! The API endpoints are `/attach` and `/detach`.
//!
//! The module defines two async functions `attach_rt` and `detach_rt` that handle the requests to the API endpoints.
//! The `router` function creates a new router and maps the API endpoints to their respective functions.

use axum::{Json, Router, extract::State, routing::post};
use log::debug;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{DbPool, RequestSession, appstate::AppState, error::ApiResult};
use tb_domain::{PartId, PartTypeId, Store, Summary};

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
    user: RequestSession,
    State(store): State<DbPool>,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    let mut store = store.begin().await?;
    debug!("attach {event:?}");

    let Event {
        part_id,
        time,
        gear,
        hook,
        all,
    } = event;

    let res = tb_domain::attach_assembly(&user, part_id, time, gear, hook, all, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

/// route for detach API
async fn detach_rt(
    user: RequestSession,
    State(store): State<DbPool>,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    let mut store = store.begin().await?;
    debug!("detach {event:?}");
    let Event {
        part_id, time, all, ..
    } = event;
    let res = tb_domain::detach_assembly(&user, part_id, time, all, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Dispose {
    part_id: PartId,
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    all: bool,
}

async fn dispose_rt(
    user: RequestSession,
    State(store): State<DbPool>,
    Json(event): Json<Dispose>,
) -> ApiResult<Summary> {
    let mut store = store.begin().await?;
    debug!("{event:?}");
    let Dispose {
        part_id: part,
        time,
        all,
    } = event;
    let res = tb_domain::dispose_assembly(&user, part, time, all, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

async fn recover_rt(
    user: RequestSession,
    State(store): State<DbPool>,
    Json(event): Json<Dispose>,
) -> ApiResult<Summary> {
    let mut store = store.begin().await?;
    debug!("Recover {event:?}");
    let Dispose {
        part_id: part, all, ..
    } = event;
    let res = tb_domain::recover_assembly(&user, part, all, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/attach", post(attach_rt))
        .route("/detach", post(detach_rt))
        .route("/dispose", post(dispose_rt))
        .route("/recover", post(recover_rt))
}
