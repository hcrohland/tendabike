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
    routing::{delete, get, post},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    DbPool, RequestUser,
    appstate::AppState,
    error::{ApiResult, AppError},
};
use tb_domain::{
    Garage, GarageId, GarageSubscription, GarageSubscriptionWithDetails, GarageWithOwner, PartId,
    Person, Store, SubscriptionId,
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
pub struct NewSubscriptionRequest {
    pub garage_id: i32,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionResponseRequest {
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterPartRequest {
    pub part_id: i32,
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
        .route("/{garage}/parts", get(get_garage_parts).post(register_part))
        .route("/{garage}/parts/{part}", delete(unregister_part))
        // Subscriptions
        .route(
            "/subscriptions",
            get(list_my_subscriptions).post(create_subscription),
        )
        .route(
            "/subscriptions/{subscription}",
            get(get_subscription).delete(cancel_subscription),
        )
        .route(
            "/subscriptions/{subscription}/approve",
            post(approve_subscription),
        )
        .route(
            "/subscriptions/{subscription}/reject",
            post(reject_subscription),
        )
        .route("/{garage}/subscriptions", get(list_garage_subscriptions))
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
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(RegisterPartRequest { part_id }): Json<RegisterPartRequest>,
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
) -> ApiResult<Vec<GarageWithOwner>> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let mut store = pool.begin().await?;
    let garages = Garage::search(query, &mut store).await?;
    let garages_with_owner = Garage::with_owner_info(garages, &mut store).await?;
    Ok(Json(garages_with_owner))
}

// Subscription handlers

async fn create_subscription(
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(NewSubscriptionRequest { garage_id, message }): Json<NewSubscriptionRequest>,
) -> Result<(StatusCode, Json<GarageSubscription>), AppError> {
    let mut store = pool.begin().await?;
    let subscription = SubscriptionId::create(garage_id.into(), message, &user, &mut store).await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(subscription)))
}

async fn list_my_subscriptions(
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<GarageSubscriptionWithDetails>> {
    let mut store = pool.begin().await?;
    let subscriptions = GarageSubscription::get_for_user(&user, &mut store).await?;
    let subscriptions_with_details =
        GarageSubscription::with_garage_details(subscriptions, &mut store).await?;
    Ok(Json(subscriptions_with_details))
}

async fn list_garage_subscriptions(
    Path(garage_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<GarageSubscription>> {
    let mut store = pool.begin().await?;
    let garage_id = GarageId::get(garage_id, &user, &mut store).await?;
    Ok(
        GarageSubscription::get_pending_for_garage(garage_id, &user, &mut store)
            .await
            .map(Json)?,
    )
}

async fn get_subscription(
    Path(subscription_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> ApiResult<GarageSubscription> {
    let mut store = pool.begin().await?;
    let subscription_id = SubscriptionId::get(subscription_id, &user, &mut store).await?;
    Ok(subscription_id.read(&user, &mut store).await.map(Json)?)
}

async fn approve_subscription(
    Path(subscription_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> ApiResult<GarageSubscription> {
    let mut store = pool.begin().await?;
    let subscription_id = SubscriptionId::get(subscription_id, &user, &mut store).await?;
    let subscription = subscription_id
        .approve(req.message, &user, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(subscription))
}

async fn reject_subscription(
    Path(subscription_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> ApiResult<GarageSubscription> {
    let mut store = pool.begin().await?;
    let subscription_id = SubscriptionId::get(subscription_id, &user, &mut store).await?;
    let subscription = subscription_id
        .reject(req.message, &user, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(subscription))
}

async fn cancel_subscription(
    Path(subscription_id): Path<i32>,
    user: RequestUser,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let subscription_id = SubscriptionId::get(subscription_id, &user, &mut store).await?;
    subscription_id.cancel(&user, &mut store).await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
