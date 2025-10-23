//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    extract::{Path, State},
    response::Redirect,
};

use crate::{DbPool, RequestUser, error::AppError};

pub(super) async fn redirect_gear(
    mut user: RequestUser,
    Path(id): Path<i32>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.get().await?;
    let uri = tb_strava::gear::strava_url(id, &mut user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act(
    user: RequestUser,
    Path(id): Path<i64>,
    State(store): State<DbPool>,
) -> Result<Redirect, AppError> {
    let mut store = store.get().await?;
    let uri = tb_strava::activity::strava_url(id, &user, &mut store)
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
