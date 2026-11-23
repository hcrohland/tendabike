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
use log::trace;
use serde::Deserialize;

use crate::{RequestSession, appstate::AppState, domain::created_entity, error::AppError};
use tb_domain::{ApiWrite, PartId, ServicePlan, ServicePlanId};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create).put(update))
        .route("/{id}", delete(delete_plan))
        .route("/{id}/snooze", post(snooze))
}

async fn create(
    user: RequestSession,
    State(state): State<AppState>,
    Json(plan): Json<ServicePlan>,
) -> Result<(StatusCode, Json<ServicePlan>), AppError> {
    trace!("ServicePlan::create");
    let outcome = state
        .api
        .write(Box::new(user), ApiWrite::ServicePlanCreate { plan })
        .await?;
    let plan = created_entity(&outcome.summary().plans, "service plan")?;
    Ok((StatusCode::CREATED, Json(plan)))
}

async fn update(
    user: RequestSession,
    State(state): State<AppState>,
    Json(plan): Json<ServicePlan>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(Box::new(user), ApiWrite::ServicePlanUpdate { plan })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_plan(
    user: RequestSession,
    State(state): State<AppState>,
    Path(id): Path<ServicePlanId>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(Box::new(user), ApiWrite::ServicePlanDelete { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The parsed body of `POST /api/plan/{id}/snooze`: the part to snooze and
/// the fraction to add to its accumulated snooze (issue #971).
#[derive(Deserialize)]
struct SnoozeBody {
    part: PartId,
    fraction: f64,
}

async fn snooze(
    user: RequestSession,
    State(state): State<AppState>,
    Path(id): Path<ServicePlanId>,
    Json(body): Json<SnoozeBody>,
) -> Result<StatusCode, AppError> {
    trace!("ServicePlan::snooze");
    state
        .api
        .write(
            Box::new(user),
            ApiWrite::ServicePlanSnooze {
                id,
                part: body.part,
                fraction: body.fraction,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
