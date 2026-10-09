//! This module contains the implementation of the attachment API endpoints.
//!
//! The attachment API allows users to attach and detach parts to other parts.
//! The API endpoints are `/attach` and `/detach`.
//!
//! The module defines two async functions `attach_rt` and `detach_rt` that handle the requests to the API endpoints.
//! The `router` function creates a new router and maps the API endpoints to their respective functions.

use axum::{Json, Router, extract::State, routing::post};
use http::StatusCode;
use log::debug;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{RequestSession, appstate::AppState, error::AppError};
use tb_domain::{ApiWrite, PartId, PartTypeId};
use tb_exec::TxnSource;
use tb_strava::{StravaSession, StravaStore};

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
async fn attach_rt<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(event): Json<Event>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    debug!("attach {event:?}");
    let Event {
        part_id,
        time,
        gear,
        hook,
        all,
    } = event;
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::AttachmentAttach {
                part: part_id,
                time,
                gear,
                hook,
                all,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// route for detach API
async fn detach_rt<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(event): Json<Event>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    debug!("detach {event:?}");
    let Event {
        part_id, time, all, ..
    } = event;
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::AttachmentDetach {
                part: part_id,
                time,
                all,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Dispose {
    part_id: PartId,
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    all: bool,
}

async fn dispose_rt<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(event): Json<Dispose>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    debug!("{event:?}");
    let Dispose {
        part_id: part,
        time,
        all,
    } = event;
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::AttachmentDispose { part, time, all },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn recover_rt<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(event): Json<Dispose>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    debug!("Recover {event:?}");
    let Dispose {
        part_id: part, all, ..
    } = event;
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::AttachmentRecover { part, all },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
    Router::new()
        .route("/attach", post(attach_rt))
        .route("/detach", post(detach_rt))
        .route("/dispose", post(dispose_rt))
        .route("/recover", post(recover_rt))
}
