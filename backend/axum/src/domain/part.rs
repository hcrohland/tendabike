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
use std::collections::HashSet;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tb_exec::TxnSource;

use crate::{
    RequestSession,
    appstate::AppState,
    domain::created_entity,
    error::{ApiResult, AppError},
};
use serde_with::serde_as;
use tb_domain::{ApiRead, ApiWrite, Error, Part, PartId, PartTypeId, ReadOutcome};
use tb_strava::StravaSession;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewPart {
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    #[serde_as(as = "Rfc3339")]
    pub purchase: OffsetDateTime,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ChangePart {
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    #[serde_as(as = "Rfc3339")]
    pub purchase: OffsetDateTime,
}

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/", post(post_part))
        .route("/{part}", get(get_part).put(put_part).delete(delete_part))
        .route("/categories", get(mycats))
}

async fn get_part<S>(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Part>
where
    S: TxnSource + Clone + 'static,
{
    match state
        .registry
        .read(&state.source, user.tb_id(), ApiRead::Part(part))
        .await?
    {
        ReadOutcome::Part(part) => Ok(Json(part)),
        // This route sent the `Part` read; any other outcome is a bug, not
        // a domain outcome.
        _ => Err(AppError::TbError(Error::AnyFailure(anyhow::anyhow!(
            "the part read did not carry its part"
        )))),
    }
}

async fn post_part<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(NewPart {
        what,
        name,
        vendor,
        model,
        purchase,
    }): Json<NewPart>,
) -> Result<(StatusCode, Json<Part>), AppError>
where
    S: TxnSource + Clone + 'static,
{
    // The write runs on the user's executor (spec §6.2); the 201 body is the
    // created part extracted from the write's `Summary`.
    let summary = state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartCreate {
                name,
                vendor,
                model,
                what,
                purchase,
            },
        )
        .await?;
    let part = created_entity(&summary.parts, "part")?;
    Ok((StatusCode::CREATED, Json(part)))
}

async fn delete_part<S>(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartDelete { id: part },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn put_part<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Path(part): Path<PartId>,
    Json(ChangePart {
        name,
        vendor,
        model,
        purchase,
    }): Json<ChangePart>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartChange {
                id: part,
                name,
                vendor,
                model,
                purchase,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn mycats<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<HashSet<PartTypeId>>
where
    S: TxnSource + Clone + 'static,
{
    match state
        .registry
        .read(&state.source, user.tb_id(), ApiRead::PartCategories)
        .await?
    {
        ReadOutcome::PartCategories(categories) => Ok(Json(categories)),
        // This route sent the `PartCategories` read; any other outcome is a
        // bug, not a domain outcome.
        _ => Err(AppError::TbError(Error::AnyFailure(anyhow::anyhow!(
            "the part categories read did not carry its categories"
        )))),
    }
}
