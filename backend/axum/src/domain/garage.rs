//! This file contains the implementation of the `garage` resource endpoints.
//!
//! The `garage` resource represents a garage where users can register their bikes
//! for maintenance management. The endpoints in this file handle HTTP requests related
//! to garage operations.
//!
//! The following endpoints are implemented:
//!
//! - `GET /`: retrieves all garages for the authenticated user
//! - `POST /`: creates a new garage
//! - `GET /{garage}`: retrieves a specific garage
//! - `PUT /{garage}`: updates an existing garage
//! - `DELETE /{garage}`: deletes a garage (only if it has no bikes)
//! - `GET /{garage}/parts`: retrieves all parts registered to a garage
//! - `POST /{garage}/parts/{part}`: registers a part to a garage
//! - `DELETE /{garage}/parts/{part}`: unregisters a part from a garage

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
use tb_domain::{
    Garage, GarageId, GarageRegistrationRequest, PartId, Person, RegistrationRequestId, Store,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewGarage {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateGarage {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewRegistrationRequest {
    pub garage_id: i32,
    pub part_id: i32,
    pub message: Option<String>,
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        // Garage CRUD
        .route("/", get(list_garages).post(create_garage))
        .route("/search", get(search_garages))
        .route(
            "/{garage}",
            get(get_garage).put(update_garage).delete(delete_garage),
        )
        .route("/{garage}/parts", get(get_garage_parts))
        .route(
            "/{garage}/parts/{part}",
            post(register_part).delete(unregister_part),
        )
        // Registration requests
        .route("/requests", get(list_my_requests).post(create_request))
        .route(
            "/requests/{request}",
            get(get_request).delete(cancel_request),
        )
        .route("/requests/{request}/approve", post(approve_request))
        .route("/requests/{request}/reject", post(reject_request))
        .route("/{garage}/requests", get(list_garage_requests))
}

async fn list_garages(user: RequestUser, State(pool): State<DbPool>) -> ApiResult<Vec<Garage>> {
    let mut store = pool.begin().await?;
    Ok(Garage::get_all_for_user(&user.get_id(), &mut store)
        .await
        .map(Json)?)
}

async fn create_garage(
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(NewGarage { name, description }): Json<NewGarage>,
) -> Result<(StatusCode, Json<Garage>), AppError> {
    let mut store = pool.begin().await?;
    let garage = GarageId::create(name, description, &user, &mut store).await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(garage)))
}

async fn get_garage(
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<Garage> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    Ok(garage_id.read(&user, &mut store).await.map(Json)?)
}

async fn update_garage(
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(UpdateGarage { name, description }): Json<UpdateGarage>,
) -> ApiResult<Garage> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    let garage = garage_id
        .update(name, description, &user, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(garage))
}

async fn delete_garage(
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    garage_id.delete(&user, &mut store).await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_garage_parts(
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<PartId>> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    Ok(garage_id.get_parts(&user, &mut store).await.map(Json)?)
}

async fn register_part(
    Path((garage_id, part_id)): Path<(i32, i32)>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    let part_id = PartId::get(part_id, &user, &mut store).await?;
    garage_id.register_part(part_id, &user, &mut store).await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unregister_part(
    Path((garage_id, part_id)): Path<(i32, i32)>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    garage_id
        .unregister_part(part_id.into(), &user, &mut store)
        .await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// Search garages
async fn search_garages(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<Garage>> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let mut store = pool.begin().await?;
    Ok(Garage::search(query, &mut store).await.map(Json)?)
}

// Registration request handlers

async fn create_request(
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(NewRegistrationRequest {
        garage_id,
        part_id,
        message,
    }): Json<NewRegistrationRequest>,
) -> Result<(StatusCode, Json<GarageRegistrationRequest>), AppError> {
    let mut store = pool.begin().await?;
    let request =
        RegistrationRequestId::create(garage_id.into(), part_id.into(), message, &user, &mut store)
            .await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(request)))
}

async fn list_my_requests(
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<GarageRegistrationRequest>> {
    let mut store = pool.begin().await?;
    Ok(GarageRegistrationRequest::get_for_user(&user, &mut store)
        .await
        .map(Json)?)
}

async fn list_garage_requests(
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<GarageRegistrationRequest>> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    Ok(
        GarageRegistrationRequest::get_pending_for_garage(garage_id, &user, &mut store)
            .await
            .map(Json)?,
    )
}

async fn get_request(
    Path(request_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<GarageRegistrationRequest> {
    let mut store = pool.begin().await?;
    let request_id = RegistrationRequestId::get(request_id, &user, &mut store).await?;
    Ok(request_id.read(&user, &mut store).await.map(Json)?)
}

async fn approve_request(
    Path(request_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<GarageRegistrationRequest> {
    let mut store = pool.begin().await?;
    let request_id = RegistrationRequestId::get(request_id, &user, &mut store).await?;
    let request = request_id.approve(&user, &mut store).await?;
    store.commit().await?;
    Ok(Json(request))
}

async fn reject_request(
    Path(request_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<GarageRegistrationRequest> {
    let mut store = pool.begin().await?;
    let request_id = RegistrationRequestId::get(request_id, &user, &mut store).await?;
    let request = request_id.reject(&user, &mut store).await?;
    store.commit().await?;
    Ok(Json(request))
}

async fn cancel_request(
    Path(request_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let request_id = RegistrationRequestId::get(request_id, &user, &mut store).await?;
    request_id.cancel(&user, &mut store).await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
