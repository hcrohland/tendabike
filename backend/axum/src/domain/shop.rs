//! This file contains the implementation of the `shop` resource endpoints.
//!
//! The `shop` resource represents a shop where users can register their bikes
//! for maintenance management. The endpoints in this file handle HTTP requests related
//! to shop operations.
//!
//! The following endpoints are implemented:
//!
//! - `GET /`: retrieves all shops for the authenticated user
//! - `POST /`: creates a new shop
//! - `GET /{shop}`: retrieves a specific shop
//! - `PUT /{shop}`: updates an existing shop
//! - `DELETE /{shop}`: deletes a shop (only if it has no bikes)
//! - `GET /{shop}/parts`: retrieves all parts registered to a shop
//! - `POST /{shop}/parts/{part}`: registers a part to a shop
//! - `DELETE /{shop}/parts/{part}`: unregisters a part from a shop

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    DbPool, RequestSession,
    appstate::AppState,
    error::{ApiResult, AppError},
};
use tb_domain::{
    Part, Session, Shop, ShopId, ShopSubscription, ShopSubscriptionWithDetails, ShopWithOwner,
    Store, SubscriptionId,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewShop {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateShop {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewSubscriptionRequest {
    pub shop_id: i32,
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
        // Shop CRUD
        .route("/", get(list_shops).post(create_shop))
        .route("/search", get(search_shops))
        .route(
            "/{shop}",
            get(get_shop).put(update_shop).delete(delete_shop),
        )
        .route("/{shop}/parts", get(get_shop_parts).post(register_part))
        .route("/{shop}/parts/{part}", delete(unregister_part))
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
        .route("/{shop}/subscriptions", get(list_shop_subscriptions))
}

async fn list_shops(session: RequestSession, State(pool): State<DbPool>) -> ApiResult<Vec<Shop>> {
    let mut store = pool.begin().await?;
    Ok(Shop::get_all_for_user(&session.user_id(), &mut store)
        .await
        .map(Json)?)
}

async fn create_shop(
    session: RequestSession,
    State(pool): State<DbPool>,
    Json(NewShop { name, description }): Json<NewShop>,
) -> Result<(StatusCode, Json<Shop>), AppError> {
    let mut store = pool.begin().await?;
    let shop = ShopId::create(name, description, session.user_id(), &mut store).await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(shop)))
}

async fn get_shop(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> ApiResult<Shop> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get_for_read(shop_id, user, &mut store).await?;
    Ok(shop_id.read(user, &mut store).await.map(Json)?)
}

async fn update_shop(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
    Json(UpdateShop { name, description }): Json<UpdateShop>,
) -> ApiResult<Shop> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get(shop_id, user, &mut store).await?;
    let shop = shop_id.update(name, description, user, &mut store).await?;
    store.commit().await?;
    Ok(Json(shop))
}

async fn delete_shop(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get(shop_id, user, &mut store).await?;
    shop_id.delete(user, &mut store).await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_shop_parts(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<Part>> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get_for_read(shop_id, user, &mut store).await?;
    Ok(shop_id.get_parts(user, &mut store).await.map(Json)?)
}

async fn register_part(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
    Json(RegisterPartRequest { part_id }): Json<RegisterPartRequest>,
) -> ApiResult<tb_domain::Summary> {
    let mut store = pool.begin().await?;
    let shop_id: ShopId = shop_id.into();
    let summary = shop_id
        .register_part(part_id.into(), &session, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(summary))
}

async fn unregister_part(
    Path((shop_id, part_id)): Path<(i32, i32)>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> ApiResult<tb_domain::Summary> {
    let mut store = pool.begin().await?;
    let shop_id: ShopId = shop_id.into();
    let summary = shop_id
        .unregister_part(part_id.into(), &session, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(summary))
}

// Search shops
async fn search_shops(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<ShopWithOwner>> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let mut store = pool.begin().await?;
    let shops = Shop::search(query, &mut store).await?;
    let shops_with_owner = Shop::with_owner_info(shops, &mut store).await?;
    Ok(Json(shops_with_owner))
}

// Subscription handlers

async fn create_subscription(
    user: RequestSession,
    State(pool): State<DbPool>,
    Json(NewSubscriptionRequest { shop_id, message }): Json<NewSubscriptionRequest>,
) -> Result<(StatusCode, Json<ShopSubscription>), AppError> {
    let mut store = pool.begin().await?;
    let subscription =
        SubscriptionId::create(shop_id.into(), message, user.user_id(), &mut store).await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(subscription)))
}

async fn list_my_subscriptions(
    session: RequestSession,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<ShopSubscriptionWithDetails>> {
    let mut store = pool.begin().await?;
    let subscriptions = ShopSubscription::get_for_user(session.user_id(), &mut store).await?;
    let subscriptions_with_details =
        ShopSubscription::with_shop_details(subscriptions, &mut store).await?;
    Ok(Json(subscriptions_with_details))
}

async fn list_shop_subscriptions(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<ShopSubscription>> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get(shop_id, user, &mut store).await?;
    Ok(
        ShopSubscription::get_pending_for_shop(shop_id, user, &mut store)
            .await
            .map(Json)?,
    )
}

async fn get_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> ApiResult<ShopSubscription> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let subscription_id = SubscriptionId::get(subscription_id, user, &mut store).await?;
    Ok(subscription_id.read(user, &mut store).await.map(Json)?)
}

async fn approve_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> ApiResult<ShopSubscription> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let subscription_id = SubscriptionId::get(subscription_id, user, &mut store).await?;
    let subscription = subscription_id
        .approve(req.message, user, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(subscription))
}

async fn reject_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> ApiResult<ShopSubscription> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let subscription_id = SubscriptionId::get(subscription_id, user, &mut store).await?;
    let subscription = subscription_id
        .reject(req.message, user, &mut store)
        .await?;
    store.commit().await?;
    Ok(Json(subscription))
}

async fn cancel_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(pool): State<DbPool>,
) -> Result<StatusCode, AppError> {
    let mut store = pool.begin().await?;
    let user = session.user_id();
    let subscription_id = SubscriptionId::get(subscription_id, user, &mut store).await?;
    subscription_id.cancel(user, &mut store).await?;
    store.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
