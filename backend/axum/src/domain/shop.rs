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
use tb_exec::TxnSource;

use crate::{
    RequestSession,
    appstate::AppState,
    domain::created_entity,
    error::{ApiResult, AppError},
};
use tb_domain::{
    ApiWrite, Part, Session, Shop, ShopId, ShopSubscription, ShopSubscriptionWithDetails,
    SubscriptionId, UserPublic,
};
use tb_strava::{StravaSession, StravaStore};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewShop {
    pub name: String,
    pub description: Option<String>,
    pub auto_approve: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateShop {
    pub name: String,
    pub description: Option<String>,
    pub auto_approve: bool,
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

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
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

async fn list_shops<S>(
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<Shop>>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    Ok(Shop::get_all_for_user(&session.user_id(), &mut store)
        .await
        .map(Json)?)
}

async fn create_shop<S>(
    session: RequestSession,
    State(state): State<AppState<S>>,
    Json(NewShop {
        name,
        description,
        auto_approve,
    }): Json<NewShop>,
) -> Result<(StatusCode, Json<Shop>), AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let summary = state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopCreate {
                name,
                description,
                auto_approve,
            },
        )
        .await?;
    let shop = created_entity(&summary.shops, "shop")?;
    Ok((StatusCode::CREATED, Json(shop)))
}

async fn get_shop<S>(
    Path(shop_id): Path<i32>,
    _session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Shop>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    // let shop_id = ShopId::get_for_read(shop_id, user, &mut store).await?;
    Ok(ShopId::from(shop_id).read(&mut store).await.map(Json)?)
}

async fn update_shop<S>(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
    Json(UpdateShop {
        name,
        description,
        auto_approve,
    }): Json<UpdateShop>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopUpdate {
                id: ShopId::from(shop_id),
                name,
                description,
                auto_approve,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_shop<S>(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopDelete {
                id: ShopId::from(shop_id),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_shop_parts<S>(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<Part>>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get_for_read(shop_id, user, &mut store).await?;
    Ok(shop_id.get_parts(user, &mut store).await.map(Json)?)
}

async fn register_part<S>(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
    Json(RegisterPartRequest { part_id }): Json<RegisterPartRequest>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopRegisterPart {
                shop: shop_id.into(),
                part: part_id.into(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unregister_part<S>(
    Path((shop_id, part_id)): Path<(i32, i32)>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopUnregisterPart {
                shop: shop_id.into(),
                part: part_id.into(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// Search shops
async fn search_shops<S>(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<(Vec<Shop>, Vec<UserPublic>)>
where
    S: TxnSource + Clone + 'static,
{
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let mut store = state.source.begin().await?;
    let shops = Shop::search(query, &mut store).await?;
    let users = Shop::get_users(&shops, &session.user_id(), &mut store).await?;
    Ok(Json((shops, users)))
}

// Subscription handlers

async fn create_subscription<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(NewSubscriptionRequest { shop_id, message }): Json<NewSubscriptionRequest>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    // A subscription is not part of a `Summary`, so this create is a plain
    // 204 mutation (spec §6.2).
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::ShopSubscriptionCreate {
                shop: shop_id.into(),
                message,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_my_subscriptions<S>(
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<ShopSubscriptionWithDetails>>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let subscriptions = ShopSubscription::get_for_user(session.user_id(), &mut store).await?;
    let subscriptions_with_details =
        ShopSubscription::with_shop_details(subscriptions, &mut store).await?;
    Ok(Json(subscriptions_with_details))
}

async fn list_shop_subscriptions<S>(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<ShopSubscriptionWithDetails>>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let user = session.user_id();
    let shop_id = ShopId::get(shop_id, user, &mut store).await?;
    Ok(
        ShopSubscription::get_pending_for_shop(shop_id, user, &mut store)
            .await
            .map(Json)?,
    )
}

async fn get_subscription<S>(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<ShopSubscription>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let user = session.user_id();
    let subscription_id = SubscriptionId::get(subscription_id, user, &mut store).await?;
    Ok(subscription_id.read(user, &mut store).await.map(Json)?)
}

async fn approve_subscription<S>(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopSubscriptionApprove {
                id: subscription_id.into(),
                message: req.message,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reject_subscription<S>(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopSubscriptionReject {
                id: subscription_id.into(),
                message: req.message,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn cancel_subscription<S>(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            session.tb_id(),
            ApiWrite::ShopSubscriptionCancel {
                id: subscription_id.into(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
