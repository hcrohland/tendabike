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

use crate::{
    error::{ApiResult, AppError},
    appstate::AppState, DbPool,
    RequestUser
};

use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use http::StatusCode;
use tb_domain::{ChangePart, NewPart, Part, PartId};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/", put(put_part).post(post_part))
        .route("/:part", get(get_part))
}

async fn get_part(Path(part): Path<i32>, user: RequestUser, State(conn): State<DbPool>) -> ApiResult<Part> {
    let mut conn = conn.get().await?;
    Ok(PartId::new(part).part(&user, &mut conn).await.map(Json)?)
}

async fn post_part(
    user: RequestUser,
    State(conn): State<DbPool>,
    Json(newpart): Json<NewPart>,
) -> Result<(StatusCode, Json<Part>), AppError> {
    let mut conn = conn.get().await?;
    let part = newpart.clone().create(&user, &mut conn).await?;
    Ok((StatusCode::CREATED, Json(part)))
}

async fn put_part(
    user: RequestUser,
    State(conn): State<DbPool>,
    Json(part): Json<ChangePart>,
) -> ApiResult<Part> {
    let mut conn = conn.get().await?;
    Ok(part.change(&user, &mut conn).await.map(Json)?)
}
