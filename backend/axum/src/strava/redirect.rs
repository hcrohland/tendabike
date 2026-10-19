//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    Json,
    extract::{Path, State},
    response::Redirect,
};
use tb_domain::{ActivityId, PartId, Session, UserId};

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, error::AppError};

pub(super) async fn redirect_gear(
    user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    // The service API's direct op (spec #558 §5.2): one transaction — the
    // caller's session built in-txn (the user-gated `strava_url` op, which
    // checks the part's ownership), then the Strava URL. The handler maps
    // `Err` → `Redirect::permanent("/")` as before.
    let uri = state
        .api
        .gear_url(user.user_id(), PartId::from(id))
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act(
    user: RequestSession,
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let uri = state
        .api
        .activity_url(user.user_id(), ActivityId::from(id))
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let uri = state
        .api
        .user_url(id.into())
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn revoke_user(
    _admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The service API's direct op (spec #558 §5.2): one transaction — the
    // target's session built in-txn, then `tb_strava::user_deauthorize`
    // (no wake, no message; the existing queue race is preserved).
    state.api.deauthorize(tbid).await?;
    Ok(Json(()))
}

pub(super) async fn deleteuser(
    _admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The service API's direct op (spec #558 §5.2): one transaction — the
    // target's session built in-txn, then `tb_strava::user_delete` (no wake,
    // no message; the existing queue race is preserved).
    state.api.delete_user(tbid).await?;
    Ok(Json(()))
}
