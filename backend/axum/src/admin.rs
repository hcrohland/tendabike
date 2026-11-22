//! The provider-neutral admin direct ops (ADR-0014): the manual sync trigger
//! (batched and per-user), the deauthorization, and the user delete.
//!
//! Each acts on a user of *either* provider: the provider layer's dispatch
//! reads the target's real provider in-transaction (`provider_of`) and does
//! the provider's work, so the route carries no provider of its own. They were
//! mounted under `/strava/…` historically (Strava was the first provider); the
//! `/admin` prefix makes the provider-neutrality explicit.
//!
//! All are admin-gated (the `AxumAdmin` extractor answers 404 to non-admins)
//! and carry the admin's acting session into the op (the domain re-checks
//! `is_admin`, defense-in-depth over the web guard).

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use http::StatusCode;
use serde::Deserialize;

use crate::{
    ApiResult, AxumAdmin, appstate::AppState, domain::user::InitialSyncQuery, error::AppError,
};
use tb_domain::UserId;

/// `GET /admin/sync?time=…&user_id=…`: the batched admin sync trigger. Queues
/// a `Sync` event per (non-disabled) user — or narrows to one via `?user_id=`.
#[derive(Deserialize)]
pub(crate) struct SyncQuery {
    time: i64,
    user_id: Option<i32>,
}

pub(crate) async fn sync_all(
    admin: AxumAdmin,
    State(state): State<AppState>,
    Query(query): Query<SyncQuery>,
) -> ApiResult<()> {
    let user_id: Option<UserId> = query.user_id.map(|u| u.into());
    // The service API's batched admin op (ADR-0014): one transaction
    // (`sync_users`) queuing a `Sync` event per (non-disabled) user — each on
    // their own provider's queue — then, per queued user, spawn the executor
    // (a no-op if already running) and wake it, so the queue drains even for
    // offline users; `user_id` narrows the trigger to one user.
    state
        .api
        .trigger_processing(Box::new(admin.session()), user_id, query.time)
        .await?;
    Ok(Json(()))
}

/// `GET /admin/sync/{id}?time=…`: the per-user sync direct op.
pub(crate) async fn sync_one(
    Path(tbid): Path<i32>,
    admin: AxumAdmin,
    State(state): State<AppState>,
    Query(query): Query<InitialSyncQuery>,
) -> Result<StatusCode, AppError> {
    let user_id: UserId = tbid.into();
    // The provider-agnostic direct op (ADR-0014): the provider layer's
    // dispatch reads the target's real provider in-transaction and enqueues a
    // `Sync` event on their provider queue. The admin's acting session rides
    // along (carried for the admin-verification check).
    state
        .provider
        .sync(Box::new(admin.session()), user_id, query.time)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /admin/disable/{id}`: the deauthorization direct op.
pub(crate) async fn disable(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch reads the target's real provider
    // in-transaction, builds the target's session in-txn, then deauthorizes
    // (no wake, no message; the existing queue race is preserved). The
    // admin's acting session rides along (carried for the admin-verification
    // check).
    state
        .provider
        .deauthorize(Box::new(admin.session()), tbid)
        .await?;
    Ok(Json(()))
}

/// `DELETE /admin/delete/{id}`: the user delete direct op.
pub(crate) async fn delete_user(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch reads the target's real provider
    // in-transaction, builds the target's session in-txn, then deletes (no
    // wake, no message; the existing queue race is preserved). The admin's
    // acting session rides along (carried for the admin-verification check).
    state
        .provider
        .delete_user(Box::new(admin.session()), tbid)
        .await?;
    Ok(Json(()))
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/sync", get(sync_all))
        .route("/sync/{id}", get(sync_one))
        .route("/disable/{id}", post(disable))
        .route("/delete/{id}", delete(delete_user))
}
