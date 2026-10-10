/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! This module contains the implementation of the Strava webhook API endpoints.
//!
//! The webhook API is used by Strava to notify Tendabike of new activities and other events.
//! The endpoints in this module handle the incoming webhook requests, validate them, and
//! process the events.
//!
//! The following endpoints are defined in this module:
//!
//! - `hooks`: The main webhook endpoint that is called by the client to process incoming events.
//! - `create_event`: An endpoint that is called by Strava to inform about a new event.
//! - `validate_subscription`: An endpoint that is called by Strava to validate the webhook subscription.
//! - `sync_api`: An endpoint that triggers a manual sync of Strava data for all users.
//! - `sync`: An endpoint that triggers a manual sync of Strava data for a specific user.
//!
//! The `create_event` endpoint is the main entry point for incoming webhook events. It is responsible
//! for validating the incoming request, extracting the event data, and storing incoming events in the
//! database. This endpoint is not meant to be called directly.
//!
//! The `hooks` endpoint is the main entry point for clients to process incoming events and return the resulting changes.
//!
//! The `validate_subscription` endpoint is called by Strava to validate the webhook subscription.
//! When a new subscription is created, Strava sends a validation request to this endpoint. The
//! endpoint must respond with the `hub.challenge` value that was sent in the request.
//!
//! The `sync_api` endpoint triggers a manual sync of Strava data for all users. This endpoint is
//! only accessible to users with the `admin` role.
//!
//! The `sync` endpoint triggers a manual sync of Strava data for a specific user. This endpoint is
//! only accessible to users with the `admin` role.
//!

use axum::{
    Json,
    extract::{Path, Query, State},
};
use log::{info, trace};
use serde_derive::{Deserialize, Serialize};

use http::StatusCode;

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, error::AppError};
use tb_domain::{ApiWrite, Error, TbResult};
use tb_exec::{Txn, TxnSource};
use tb_strava::StravaSession;
use tb_strava::StravaStore;
use tb_strava::event::InEvent;

#[derive(Debug, Deserialize, Serialize)]
pub struct Hub {
    #[serde(rename = "hub.mode")]
    #[serde(skip_serializing)]
    mode: String,
    #[serde(rename = "hub.challenge")]
    challenge: String,
    #[serde(rename = "hub.verify_token")]
    #[serde(skip_serializing)]
    verify_token: String,
}

impl Hub {
    fn validate(self) -> TbResult<Hub> {
        if self.verify_token != VERIFY_TOKEN {
            return Err(Error::BadRequest(format!(
                "Unknown verify token {}",
                self.verify_token
            )));
        };
        if self.mode != "subscribe" {
            return Err(Error::BadRequest(format!("Unknown mode {}", self.mode)));
        };
        Ok(self)
    }
}

const VERIFY_TOKEN: &str = "tendabike_strava";

pub(crate) async fn create_event<S>(
    State(state): State<AppState<S>>,
    Json(event): axum::extract::Json<InEvent>,
) -> ApiResult<()>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    trace!("Received {event:#?}");
    let mut store = state.source.begin().await?;
    let user_id = event.accept(&mut store).await?;
    store.commit().await?;
    // Fire the in-memory wake signal (spec §4.4): a pure signal to a
    // running executor — it never spawns one, and the DB queue is the
    // source of truth, so a missed wake only delays the event to the next
    // spawn.
    if let Some(user_id) = user_id {
        state.registry.wake(user_id).await;
    }
    Ok(Json(()))
}

pub(super) async fn validate_subscription(Query(hub): Query<Hub>) -> ApiResult<Hub> {
    info!("Received validation callback {hub:?}");
    Ok(hub.validate().map(Json)?)
}

#[derive(Deserialize)]
pub(super) struct SyncQuery {
    time: i64,
    user_id: Option<i32>,
    #[serde(default)]
    migrate: bool,
}

pub(super) async fn sync_api<S>(
    _u: AxumAdmin,
    State(state): State<AppState<S>>,
    Query(query): Query<SyncQuery>,
) -> ApiResult<()>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    let user_id: Option<tb_domain::UserId> = query.user_id.map(|u| u.into());
    let res = tb_strava::event::sync_users(user_id, query.time, query.migrate, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;

    Ok(res)
}

pub(super) async fn sync<S>(
    Path(tbid): Path<i32>,
    _admin: AxumAdmin,
    State(state): State<AppState<S>>,
    Query(query): Query<InitialSyncQuery>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let user_id: tb_domain::UserId = tbid.into();
    // The executor's event queue is keyed by the Strava id, so read the
    // administered user's Strava id first (the admin gate is `AxumAdmin`).
    let mut store = state.source.begin().await?;
    let strava_id = store.stravauser_get_by_tbid(user_id).await?.strava_id();
    store.commit().await?;
    // Enqueue the sync, wake the user's executor, and await its completion
    // (spec §6.4).
    state
        .registry
        .sync(&state.source, user_id, strava_id, query.time)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(super) struct InitialSyncQuery {
    #[serde(default)]
    time: i64,
}

/// Trigger initial sync for a user
/// This endpoint allows users to trigger their first activity sync after registration.
/// It can only be called once - if the user has already completed initial sync, it returns an error.
/// Returns the updated user object.
pub(crate) async fn trigger_initial_sync<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
    Query(query): Query<InitialSyncQuery>,
) -> ApiResult<tb_domain::User>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    // The domain's status guard and completion update run on the user's
    // executor (spec §6.2); the `User` is not part of a `Summary`, so the
    // 200 body reads it back after the write succeeds.
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::UserOnboardingSync { time: query.time },
        )
        .await?;

    // The Strava-side event is a web-layer concern: queue it and fire the
    // wake signal to a running executor to consume it (spec §4.6). The wake
    // never spawns and cannot fail — the queue is the source of truth.
    let mut store = state.source.begin().await?;
    tb_strava::event::insert_sync(user.strava_id(), query.time, false, &mut store).await?;
    store.commit().await?;
    state.registry.wake(user.tb_id()).await;

    let mut store = state.source.begin().await?;
    Ok(Json(user.tb_id().read(&mut store).await?))
}

/// Postpone initial sync for a user
/// This endpoint allows users to postpone the initial activity sync.
/// It can only be called if the user is still in pending status.
/// Returns the updated user object.
pub(crate) async fn postpone_initial_sync<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<tb_domain::User>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    // The domain's status guard and postponement run on the user's executor
    // (spec §6.2); the 200 body reads the `User` back after the write
    // succeeds.
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::UserOnboardingPostpone,
        )
        .await?;

    let mut store = state.source.begin().await?;
    Ok(Json(user.tb_id().read(&mut store).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub(mode: &str, challenge: &str, verify_token: &str) -> Hub {
        Hub {
            mode: mode.to_string(),
            challenge: challenge.to_string(),
            verify_token: verify_token.to_string(),
        }
    }

    #[test]
    fn validate_accepts_valid_subscription() {
        let hub = hub("subscribe", "xyz", VERIFY_TOKEN)
            .validate()
            .expect("valid");
        assert_eq!(hub.challenge, "xyz");
    }

    #[test]
    fn validate_rejects_unknown_token() {
        let hub = hub("subscribe", "xyz", "wrong_token");
        assert!(matches!(hub.validate(), Err(Error::BadRequest(_))));
    }

    #[test]
    fn validate_rejects_unknown_mode() {
        let hub = hub("unsubscribe", "xyz", VERIFY_TOKEN);
        assert!(matches!(hub.validate(), Err(Error::BadRequest(_))));
    }
}
