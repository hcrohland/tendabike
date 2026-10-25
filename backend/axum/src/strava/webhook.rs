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

use crate::{
    ApiResult, AxumAdmin, RequestSession, appstate::AppState, domain::result_mismatch,
    error::AppError,
};
use tb_domain::{ApiRead, ApiWrite, Error, ReadOutcome, TbResult};
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

pub(crate) async fn create_event(
    State(state): State<AppState>,
    Json(event): axum::extract::Json<InEvent>,
) -> ApiResult<()> {
    trace!("Received {event:#?}");
    // The service API's direct op (spec #558 §5.2): one transaction
    // (`InEvent::accept` — the queue insert, or the athlete-deauth
    // branch), then the wake if a user was queued (fire-and-forget). A
    // failed ingest 500s to Strava (its own retry policy applies).
    state.api.accept_event(event).await?;
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

pub(super) async fn sync_api(
    admin: AxumAdmin,
    State(state): State<AppState>,
    Query(query): Query<SyncQuery>,
) -> ApiResult<()> {
    let user_id: Option<tb_domain::UserId> = query.user_id.map(|u| u.into());
    // The service API's direct op (spec #558 §5.2): one transaction
    // (`sync_users`) queuing a `Sync` event per (non-disabled) user, then —
    // per queued user — spawn the executor (a no-op if already running) and
    // wake it, so the queue drains even for offline users; `user_id` narrows
    // the trigger to one user (the route's `?user_id=` query).
    state
        .api
        .trigger_processing(
            Box::new(admin.session()),
            user_id,
            query.time,
            query.migrate,
        )
        .await?;
    Ok(Json(()))
}

pub(super) async fn sync(
    Path(tbid): Path<i32>,
    admin: AxumAdmin,
    State(state): State<AppState>,
    Query(query): Query<InitialSyncQuery>,
) -> Result<StatusCode, AppError> {
    let user_id: tb_domain::UserId = tbid.into();
    // The service API's direct op (spec #558 §5.2): read the target's
    // Strava id, enqueue a `Sync` event on the `strava_events` queue, wake
    // the executor, and await that event's completion (unbounded — the
    // spec'd behavior for the admin endpoint), with the queue-fallback
    // read only on a lost broadcast. The admin's acting session rides
    // along (carried for the admin-verification `tb_strava` change).
    state
        .api
        .sync(Box::new(admin.session()), user_id, query.time)
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
pub(crate) async fn trigger_initial_sync(
    user: RequestSession,
    State(state): State<AppState>,
    Query(query): Query<InitialSyncQuery>,
) -> ApiResult<tb_domain::User> {
    // The domain's status guard and completion update — **and** the
    // Strava-side `insert_sync` — run on the user's executor in one
    // loop-owned transaction (spec #558 §5.1: one message = one loop-owned
    // transaction containing the message's entire work, domain + Strava).
    // The 200 body reads the `User` record back (the light record-only
    // read — no `Summary` computation needed).
    state
        .api
        .write(
            Box::new(user.clone()),
            ApiWrite::UserOnboardingSync { time: query.time },
        )
        .await?;
    match state.api.read(Box::new(user), ApiRead::UserRecord).await? {
        ReadOutcome::UserRecord(user) => Ok(Json(user)),
        _ => Err(result_mismatch("user", "user record")),
    }
}

/// Postpone initial sync for a user
/// This endpoint allows users to postpone the initial activity sync.
/// It can only be called if the user is still in pending status.
/// Returns the updated user object.
pub(crate) async fn postpone_initial_sync(
    user: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<tb_domain::User> {
    // The domain's status guard and postponement run on the user's
    // executor (spec §6.2); the 200 body reads the `User` record back
    // (the light record-only read — no `Summary` computation needed).
    state
        .api
        .write(Box::new(user.clone()), ApiWrite::UserOnboardingPostpone)
        .await?;
    match state.api.read(Box::new(user), ApiRead::UserRecord).await? {
        ReadOutcome::UserRecord(user) => Ok(Json(user)),
        _ => Err(result_mismatch("user", "user record")),
    }
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
