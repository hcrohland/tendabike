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

use crate::{ApiResult, RequestSession, appstate::AppState, error::AppError};
use tb_domain::{PartId, Service, ServiceId, ServicePlanId, Summary};
use tb_exec::{Txn, TxnSource};

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>> {
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
) -> Result<(StatusCode, Json<Summary>), AppError>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    part_id.checkuser(&user, &mut store).await?;
    let summary = Service::create(part_id, time, name, notes, None, plans, &mut store).await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(summary)))
}

async fn update<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(service): Json<Service>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let res = service.update(&user, &mut store).await.map(Json)?;
    store.commit().await?;
    Ok(res)
}

async fn delete_service<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Path(id): Path<ServiceId>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let res = id.delete(&user, &mut store).await.map(Json)?;
    store.commit().await?;
    Ok(res)
}

async fn redo<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(service): Json<Service>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let res = service.redo(&user, &mut store).await.map(Json)?;
    store.commit().await?;
    Ok(res)
}
