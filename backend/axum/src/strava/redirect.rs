//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    Json,
    extract::{Path, State},
    response::Redirect,
};
use tb_domain::{Store, UserId};

use crate::{ApiResult, AxumAdmin, DbPool, RequestSession, error::AppError};

pub(super) async fn redirect_gear(
    mut user: RequestSession,
    Path(id): Path<i32>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.begin().await?;
    let uri = tb_strava::gear::strava_url(id, &mut user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act(
    user: RequestSession,
    Path(id): Path<i64>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.begin().await?;
    let uri = tb_strava::activity::strava_url(id, &user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user(
    Path(id): Path<i32>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.begin().await?;
    let uri = tb_strava::strava_url(id, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn revoke_user(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(pool): State<DbPool>,
) -> ApiResult<()> {
    let mut store = pool.begin().await?;
    let mut user = RequestSession::create_from_id(admin, tbid, &mut store).await?;
    let res = tb_strava::user_deauthorize(&mut user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

pub(super) async fn deleteuser(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(pool): State<DbPool>,
) -> ApiResult<()> {
    let mut store = pool.begin().await?;
    let mut user = RequestSession::create_from_id(admin, tbid, &mut store).await?;
    let res = tb_strava::user_delete(&mut user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}
