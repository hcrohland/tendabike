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
    routing::{get, post},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    DbPool, RequestUser,
    appstate::AppState,
    error::{ApiResult, AppError},
};
use serde_with::serde_as;
use tb_domain::{Part, PartId, PartTypeId};
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

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(post_part))
        .route("/{part}", get(get_part).put(put_part).delete(delete_part))
}

async fn get_part(
    Path(part): Path<PartId>,
    user: RequestUser,
    State(store): State<DbPool>,
) -> ApiResult<Part> {
    let mut store = store.get().await?;
    Ok(part.part(&user, &mut store).await.map(Json)?)
}

async fn post_part(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(NewPart {
        what,
        name,
        vendor,
        model,
        purchase,
    }): Json<NewPart>,
) -> Result<(StatusCode, Json<Part>), AppError> {
    let mut store = store.get().await?;
    let part = Part::create(name, vendor, model, what, None, purchase, &user, &mut store).await?;
    Ok((StatusCode::CREATED, Json(part)))
}

async fn delete_part(
    Path(part): Path<PartId>,
    user: RequestUser,
    State(store): State<DbPool>,
) -> ApiResult<PartId> {
    let mut store = store.get().await?;
    Ok(part.delete(&user, &mut store).await.map(Json)?)
}

async fn put_part(
    user: RequestUser,
    State(store): State<DbPool>,
    Path(part): Path<PartId>,
    Json(ChangePart {
        name,
        vendor,
        model,
        purchase,
    }): Json<ChangePart>,
) -> ApiResult<Part> {
    let mut store = store.get().await?;

    Ok(part
        .change(name, vendor, model, purchase, &user, &mut store)
        .await
        .map(Json)?)
}
