//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    Json,
    extract::{Path, State},
    response::Redirect,
};
use tb_domain::UserId;

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, error::AppError};
use tb_exec::{Txn, TxnSource};
use tb_strava::StravaStore;

pub(super) async fn redirect_gear<S>(
    mut user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState<S>>,
) -> Result<Redirect, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    let uri = tb_strava::gear::strava_url(id, &mut user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act<S>(
    user: RequestSession,
    Path(id): Path<i64>,
    State(state): State<AppState<S>>,
) -> Result<Redirect, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    let uri = tb_strava::activity::strava_url(id, &user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user<S>(
    Path(id): Path<i32>,
    State(state): State<AppState<S>>,
) -> Result<Redirect, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    let uri = tb_strava::strava_url(id, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn revoke_user<S>(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState<S>>,
) -> ApiResult<()>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    let mut user = RequestSession::create_from_id(admin, tbid, &mut store).await?;
    let res = tb_strava::user_deauthorize(&mut user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

pub(super) async fn deleteuser<S>(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState<S>>,
) -> ApiResult<()>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    let mut user = RequestSession::create_from_id(admin, tbid, &mut store).await?;
    let res = tb_strava::user_delete(&mut user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}
