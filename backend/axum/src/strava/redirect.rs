//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    Json,
    extract::{Path, State},
    response::Redirect,
};
use tb_domain::{PartId, UserId};

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, error::AppError};

pub(super) async fn redirect_gear(
    user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    // The service API's direct op (spec #558 §5.2): one transaction — the
    // `strava_url` op against the acting session (which checks the part's
    // ownership), then the Strava URL. The handler maps `Err` →
    // `Redirect::permanent("/")` as before.
    let uri = state
        .strava
        .gear_url(Box::new(user), PartId::from(id))
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let uri = state
        .strava
        .user_url(id.into())
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn revoke_user(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch reads the target's real provider
    // in-transaction, builds the target's session in-txn, then deauthorizes
    // (no wake, no message; the existing queue race is preserved). A
    // per-provider route acting on a user of the other provider follows the
    // stated identity rule. The admin's acting session rides along (carried
    // for the admin-verification `tb_provider::strava` change).
    state
        .provider
        .deauthorize(Box::new(admin.session()), tbid)
        .await?;
    Ok(Json(()))
}

pub(super) async fn deleteuser(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch reads the target's real provider
    // in-transaction, builds the target's session in-txn, then deletes (no
    // wake, no message; the existing queue race is preserved). A per-provider
    // route acting on a user of the other provider follows the stated
    // identity rule. The admin's acting session rides along (carried for the
    // admin-verification `tb_provider::strava` change).
    state
        .provider
        .delete_user(Box::new(admin.session()), tbid)
        .await?;
    Ok(Json(()))
}
