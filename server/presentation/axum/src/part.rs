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
    user::RUser,
    AppDbConn,
};

use axum::{
    extract::Path,
    routing::{get, put},
    Json, Router,
};
use http::StatusCode;
use kernel::domain::{ChangePart, NewPart, Part, PartId};

pub(crate) fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", put(put_part).post(post_part))
        .route("/:part", get(get_part))
        .with_state(state)
}

async fn get_part(Path(part): Path<i32>, user: RUser, mut conn: AppDbConn) -> ApiResult<Part> {
    Ok(PartId::new(part).part(&user, &mut conn).map(Json)?)
}

async fn post_part(
    user: RUser,
    mut conn: AppDbConn,
    Json(newpart): Json<NewPart>,
) -> Result<(StatusCode, Json<Part>), AppError> {
    let part = newpart.clone().create(&user, &mut conn)?;
    Ok((StatusCode::CREATED, Json(part)))
}

async fn put_part(
    user: RUser,
    mut conn: AppDbConn,
    part: String,
) -> ApiResult<Part> {
    dbg!(&part);
    let part = serde_json::from_str::<ChangePart>(&part)?;
    Ok(part.change(&user, &mut conn).map(Json)?)
}
