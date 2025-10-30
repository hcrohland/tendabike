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

use async_session::log::trace;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, post},
};
use http::StatusCode;

use crate::{ApiResult, DbPool, RequestUser, appstate::AppState, error::AppError};
use tb_domain::{Service, ServicePlan, ServicePlanId, Store};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create).put(update))
        .route("/{id}", delete(delete_plan))
}

async fn create(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(plan): Json<ServicePlan>,
) -> Result<(StatusCode, Json<ServicePlan>), AppError> {
    trace!("ServicePlan::create");
    let mut store = store.begin().await?;
    let summary = plan.create(&user, &mut store).await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(summary)))
}

async fn update(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(plan): Json<ServicePlan>,
) -> ApiResult<ServicePlan> {
    let mut store = store.begin().await?;
    let res = plan.update(&user, &mut store).await.map(Json)?;
    store.commit().await?;
    Ok(res)
}

async fn delete_plan(
    user: RequestUser,
    State(pool): State<DbPool>,
    Path(id): Path<ServicePlanId>,
) -> ApiResult<Vec<Service>> {
    let mut store = pool.begin().await?;
    let res = id.delete(&user, &mut store).await.map(Json)?;
    store.commit().await?;
    Ok(res)
}
