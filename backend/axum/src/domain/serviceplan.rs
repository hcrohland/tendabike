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

use crate::{RequestSession, appstate::AppState, domain::created_entity, error::AppError};
use tb_domain::{ApiWrite, ServicePlan, ServicePlanId};
use tb_exec::TxnSource;
use tb_strava::{StravaSession, StravaStore};

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
    Router::new()
        .route("/", post(create).put(update))
        .route("/{id}", delete(delete_plan))
}

async fn create<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(plan): Json<ServicePlan>,
) -> Result<(StatusCode, Json<ServicePlan>), AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    trace!("ServicePlan::create");
    let summary = state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ServicePlanCreate { plan },
        )
        .await?;
    let plan = created_entity(&summary.plans, "service plan")?;
    Ok((StatusCode::CREATED, Json(plan)))
}

async fn update<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(plan): Json<ServicePlan>,
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
            ApiWrite::ServicePlanUpdate { plan },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_plan<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Path(id): Path<ServicePlanId>,
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
            ApiWrite::ServicePlanDelete { id },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
