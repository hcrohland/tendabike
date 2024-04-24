//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    extract::{Path, State},
    response::Redirect,
    Json,
};

use crate::{error::AppError, ApiResult, AxumAdmin, DbPool, RequestUser};

pub(super) async fn redirect_gear(
    Path(id): Path<i32>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.get().await?;
    let uri = tb_strava::gear::strava_url(id, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act(
    Path(id): Path<i32>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.get().await?;
    let uri = tb_strava::activity::strava_url(id, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user(
    Path(id): Path<i32>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.get().await?;
    let uri = tb_strava::strava_url(id, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn revoke_user(
    admin: AxumAdmin,
    Path(tbid): Path<i32>,
    State(pool): State<DbPool>,
) -> ApiResult<()> {
    let mut store = pool.get().await?;
    let store = &mut store;
    let mut user = RequestUser::create_from_id(admin, tbid.into(), store).await?;
    Ok(tb_strava::user_disable(&mut user, store).await.map(Json)?)
}
