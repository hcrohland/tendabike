//! This file contains the implementation of the `part` resource endpoints.
//!
//! The `part` resource represents a part of a bike. It can be used to create, read, update and delete
//! parts of a bike. The endpoints in this file are used to handle HTTP requests related to the `part`
//! resource.
//!
//! The endpoints are implemented using the Axum web framework.
//!
//! The following endpoints are implemented:
//!
//! - `POST /`: creates a new part
//! - `PUT /`: updates an existing part
//! - `GET /{part}`: retrieves a specific part
//!
//! The endpoints use the `AppDbConn` type to interact with the database. The `RUser` type is used to
//! represent the authenticated user making the request.
//!
//! The `Part`, `NewPart` and `ChangePart` types are used to represent parts in different stages of
//! their lifecycle.
//!
//! The `router` function returns an Axum `Router` that can be mounted in a larger application.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, post},
};
use http::StatusCode;
use serde_derive::Deserialize;
use time::OffsetDateTime;

use crate::{RequestSession, appstate::AppState, domain::created_entity, error::AppError};
use tb_domain::{ApiWrite, PartId, Service, ServiceId, ServicePlanId};
use tb_exec::TxnSource;
use tb_strava::{StravaSession, StravaStore};

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
    Router::new()
        .route("/", post(create).put(update))
        .route("/{id}", delete(delete_service))
        .route("/redo", post(redo))
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct NewService {
    part_id: PartId,
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    name: String,
    notes: String,
    plans: Vec<ServicePlanId>,
}
async fn create<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(NewService {
        part_id,
        time,
        name,
        notes,
        plans,
    }): Json<NewService>,
) -> Result<(StatusCode, Json<Service>), AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let summary = state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ServiceCreate {
                part: part_id,
                time,
                name,
                notes,
                plans,
            },
        )
        .await?;
    let service = created_entity(&summary.services, "service")?;
    Ok((StatusCode::CREATED, Json(service)))
}

async fn update<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(service): Json<Service>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ServiceUpdate { service },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_service<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Path(id): Path<ServiceId>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(&state.source, user.tb_id(), ApiWrite::ServiceDelete { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn redo<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(service): Json<Service>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ServiceRedo { service },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
