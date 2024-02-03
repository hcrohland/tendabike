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
//! - `GET /:part`: retrieves a specific part
//!
//! The endpoints use the `AppDbConn` type to interact with the database. The `RUser` type is used to
//! represent the authenticated user making the request.
//!
//! The `Part`, `NewPart` and `ChangePart` types are used to represent parts in different stages of
//! their lifecycle.
//!
//! The `router` function returns an Axum `Router` that can be mounted in a larger application.

use axum::{extract::State, routing::post, Json, Router};
use http::StatusCode;
use serde_derive::Deserialize;
use time::OffsetDateTime;

use crate::{appstate::AppState, error::AppError, DbPool, RequestUser};
use tb_domain::{PartId, Service, Summary};

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/", post(create))
    // .route("/:part", get(get_part))
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct NewService {
    part_id: PartId,
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    name: String,
    notes: String,
}
async fn create(
    user: RequestUser,
    State(store): State<DbPool>,
    Json(NewService {
        part_id,
        time,
        name,
        notes,
    }): Json<NewService>,
) -> Result<(StatusCode, Json<Summary>), AppError> {
    let mut store = store.get().await?;
    part_id.checkuser(&user, &mut store).await?;
    let summary = Service::create(part_id, time, name, notes, &mut store).await?;
    Ok((StatusCode::CREATED, Json(summary)))
}
