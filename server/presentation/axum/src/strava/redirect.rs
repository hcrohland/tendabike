//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{response::Redirect, extract::{Path, State}};

use crate::{DbPool, error::AppError};

pub(super) async fn redirect_gear(Path(id): Path<i32>, State(conn): State<DbPool>) -> Result<Redirect, AppError> {
    let mut conn = conn.get().await?;
    let uri = tb_strava::gear::strava_url(id, &mut conn).await.unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act(Path(id): Path<i32>, State(conn): State<DbPool>) -> Result<Redirect, AppError> {
    let mut conn = conn.get().await?;
    let uri = tb_strava::activity::strava_url(id, &mut conn).await.unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user(Path(id): Path<i32>, State(conn): State<DbPool>) -> Result<Redirect, AppError> {
    let mut conn = conn.get().await?;
    let uri = tb_strava::strava_url(id, &mut conn).await.unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}