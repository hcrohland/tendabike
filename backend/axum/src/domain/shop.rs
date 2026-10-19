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
    RequestSession,
    appstate::AppState,
    domain::{created_entity, result_mismatch},
    error::{ApiResult, AppError},
};
use tb_domain::{
    ApiRead, ApiWrite, Error, Part, ReadOutcome, Session, Shop, ShopId, ShopSubscription,
    ShopSubscriptionWithDetails, SubscriptionId, UserPublic,
};

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

async fn list_shops(
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<Vec<Shop>> {
    match state.api.read(session.user_id(), ApiRead::Shops).await? {
        ReadOutcome::Shops(shops) => Ok(Json(shops)),
        _ => Err(result_mismatch("shops", "shops")),
    }
}

async fn create_shop(
    session: RequestSession,
    State(state): State<AppState>,
    Json(NewShop {
        name,
        description,
        auto_approve,
    }): Json<NewShop>,
) -> Result<(StatusCode, Json<Shop>), AppError> {
    let outcome = state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopCreate {
                name,
                description,
                auto_approve,
            },
        )
        .await?;
    let shop = created_entity(&outcome.summary().shops, "shop")?;
    Ok((StatusCode::CREATED, Json(shop)))
}

async fn get_shop(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<Shop> {
    match state
        .api
        .read(session.user_id(), ApiRead::Shop(ShopId::from(shop_id)))
        .await?
    {
        ReadOutcome::Shop(shop) => Ok(Json(shop)),
        _ => Err(result_mismatch("shop", "shop")),
    }
}

async fn update_shop(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
    Json(UpdateShop {
        name,
        description,
        auto_approve,
    }): Json<UpdateShop>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
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

async fn delete_shop(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopDelete {
                id: ShopId::from(shop_id),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_shop_parts(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<Vec<Part>> {
    match state
        .api
        .read(session.user_id(), ApiRead::ShopParts(ShopId::from(shop_id)))
        .await?
    {
        ReadOutcome::ShopParts(parts) => Ok(Json(parts)),
        _ => Err(result_mismatch("shop parts", "parts")),
    }
}

async fn register_part(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
    Json(RegisterPartRequest { part_id }): Json<RegisterPartRequest>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopRegisterPart {
                shop: shop_id.into(),
                part: part_id.into(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unregister_part(
    Path((shop_id, part_id)): Path<(i32, i32)>,
    session: RequestSession,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopUnregisterPart {
                shop: shop_id.into(),
                part: part_id.into(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// Search shops
async fn search_shops(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<(Vec<Shop>, Vec<UserPublic>)> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    match state
        .api
        .read(session.user_id(), ApiRead::ShopSearch(query.to_string()))
        .await?
    {
        ReadOutcome::ShopSearch { shops, users } => Ok(Json((shops, users))),
        _ => Err(result_mismatch("shop search", "shops and users")),
    }
}

// Subscription handlers

async fn create_subscription(
    user: RequestSession,
    State(state): State<AppState>,
    Json(NewSubscriptionRequest { shop_id, message }): Json<NewSubscriptionRequest>,
) -> Result<(StatusCode, Json<ShopSubscription>), AppError> {
    state
        .api
        .write(
            user.user_id(),
            ApiWrite::ShopSubscriptionCreate {
                shop: shop_id.into(),
                message,
            },
        )
        .await?;

    // A `ShopSubscription` is in no `Summary`, so the 201 body reads it back
    // after the write succeeds (spec §6.2, the onboarding pattern): the
    // create guard allows at most one subscription per (user, shop), so the
    // one for this shop is the one we just created. The read is the user's
    // own subscriptions (the mine route's read); the 201 body projects the
    // bare `ShopSubscription` from the with-details composite.
    let shop = ShopId::from(shop_id);
    let subscriptions = match state
        .api
        .read(user.user_id(), ApiRead::ShopSubscriptions(None))
        .await?
    {
        ReadOutcome::ShopSubscriptions(subscriptions) => subscriptions,
        _ => return Err(result_mismatch("shop subscriptions", "subscriptions")),
    };
    let subscription = subscriptions
        .into_iter()
        .find(|s| s.shop_id == shop)
        .ok_or_else(|| Error::NotFound("subscription not found after create".to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(ShopSubscription {
            id: subscription.id,
            shop_id: subscription.shop_id,
            user_id: subscription.user_id,
            status: subscription.status,
            message: subscription.message,
            response_message: subscription.response_message,
            created_at: subscription.created_at,
            updated_at: subscription.updated_at,
        }),
    ))
}

async fn list_my_subscriptions(
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<Vec<ShopSubscriptionWithDetails>> {
    match state
        .api
        .read(session.user_id(), ApiRead::ShopSubscriptions(None))
        .await?
    {
        ReadOutcome::ShopSubscriptions(subscriptions) => Ok(Json(subscriptions)),
        _ => Err(result_mismatch("shop subscriptions", "subscriptions")),
    }
}

async fn list_shop_subscriptions(
    Path(shop_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<Vec<ShopSubscriptionWithDetails>> {
    match state
        .api
        .read(
            session.user_id(),
            ApiRead::ShopSubscriptions(Some(ShopId::from(shop_id))),
        )
        .await?
    {
        ReadOutcome::ShopSubscriptions(subscriptions) => Ok(Json(subscriptions)),
        _ => Err(result_mismatch("shop subscriptions", "subscriptions")),
    }
}

async fn get_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<ShopSubscription> {
    match state
        .api
        .read(
            session.user_id(),
            ApiRead::ShopSubscription(SubscriptionId::from(subscription_id)),
        )
        .await?
    {
        ReadOutcome::ShopSubscription(subscription) => Ok(Json(subscription)),
        _ => Err(result_mismatch("shop subscription", "subscription")),
    }
}

async fn approve_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopSubscriptionApprove {
                id: subscription_id.into(),
                message: req.message,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reject_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
    Json(req): Json<SubscriptionResponseRequest>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopSubscriptionReject {
                id: subscription_id.into(),
                message: req.message,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn cancel_subscription(
    Path(subscription_id): Path<i32>,
    session: RequestSession,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    state
        .api
        .write(
            session.user_id(),
            ApiWrite::ShopSubscriptionCancel {
                id: subscription_id.into(),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
