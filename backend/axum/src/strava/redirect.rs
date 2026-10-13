//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{
    Json,
    extract::{Path, State},
    response::Redirect,
};
use tb_domain::{ConcreteDomain, Store, UserId};

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, error::AppError};
use tb_exec::{Txn, TxnSource};
use tb_strava::StravaStore;

pub(super) async fn redirect_gear<St, S>(
    mut user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState<St, S>>,
) -> Result<Redirect, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let uri = tb_strava::gear::strava_url::<ConcreteDomain<St>, _>(id, &mut user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_act<St, S>(
    user: RequestSession,
    Path(id): Path<i64>,
    State(state): State<AppState<St, S>>,
) -> Result<Redirect, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let uri = tb_strava::activity::strava_url::<ConcreteDomain<St>, _>(id, &user, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn redirect_user<St, S>(
    Path(id): Path<i32>,
    State(state): State<AppState<St, S>>,
) -> Result<Redirect, AppError>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let uri = tb_strava::strava_url::<ConcreteDomain<St>>(id, &mut store)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

pub(super) async fn revoke_user<St, S>(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState<St, S>>,
) -> ApiResult<()>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let mut user = RequestSession::create_from_id(admin, tbid, &mut store).await?;
    let res = tb_strava::user_deauthorize::<ConcreteDomain<St>, _>(&mut user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}

pub(super) async fn deleteuser<St, S>(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState<St, S>>,
) -> ApiResult<()>
where
    St: Store + 'static + StravaStore<ConcreteDomain<St>> + Txn<ConcreteDomain<St>>,
    S: TxnSource<ConcreteDomain<St>> + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let mut user = RequestSession::create_from_id(admin, tbid, &mut store).await?;
    let res = tb_strava::user_delete::<ConcreteDomain<St>, _>(&mut user, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
}
