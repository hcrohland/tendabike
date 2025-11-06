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

use crate::{ApiResult, AxumAdmin, DbPool, RequestUser};
use tb_domain::{Error, OnboardingStatus, Store, Summary, TbResult, UserStore};
use tb_strava::StravaPerson;
use tb_strava::event::{InEvent, process};

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

pub(crate) async fn hooks(
    mut user: RequestUser,
    State(store): State<DbPool>,
) -> ApiResult<Summary> {
    let mut store = store.begin().await?;
    let res = process(&mut user, &mut store).await;
    store.commit().await?;
    Ok(Json(res?))
}

pub(crate) async fn create_event(
    State(store): State<DbPool>,
    Json(event): axum::extract::Json<InEvent>,
) -> ApiResult<()> {
    trace!("Received {event:#?}");
    let mut store = store.begin().await?;
    event.accept(&mut store).await?;
    store.commit().await?;
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
    _u: AxumAdmin,
    State(store): State<DbPool>,
    Query(query): Query<SyncQuery>,
) -> ApiResult<()> {
    let mut store = store.begin().await?;
    let user_id: Option<tb_domain::UserId> = query.user_id.map(|u| u.into());
    let res = tb_strava::event::sync_users(user_id, query.time, query.migrate, &mut store)
        .await
        .map(Json)?;
    store.commit().await?;

    Ok(res)
}

pub(super) async fn sync(
    Path(tbid): Path<i32>,
    admin: AxumAdmin,
    State(store): State<DbPool>,
) -> ApiResult<Summary> {
    let mut store = store.begin().await?;
    let mut user = RequestUser::create_from_id(admin, tbid.into(), &mut store).await?;
    let res = process(&mut user, &mut store).await.map_err(|e| match e {
        Error::NotAuth(_) => Error::AnyFailure(anyhow::anyhow!("User not authenticated at Strava")),
        err => err,
    })?;
    store.commit().await?;
    Ok(Json(res))
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
    user: RequestUser,
    State(store): State<DbPool>,
    Query(query): Query<InitialSyncQuery>,
) -> ApiResult<tb_domain::User> {
    let mut store = store.begin().await?;

    // Check if user has already completed initial sync
    let user_data = user.tb_id().read(&mut store).await?;
    if user_data.onboarding_status.is_initial_sync_completed() {
        return Err(Error::BadRequest("Initial sync already triggered".to_string()).into());
    }

    // Insert sync event
    tb_strava::event::insert_sync(user.strava_id(), query.time, false, &mut store).await?;

    // Mark initial sync as completed and return updated user
    let updated_user = store
        .update_onboarding_status(&user.tb_id(), OnboardingStatus::Completed)
        .await?;

    store.commit().await?;
    Ok(Json(updated_user))
}

/// Postpone initial sync for a user
/// This endpoint allows users to postpone the initial activity sync.
/// It can only be called if the user is still in pending status.
/// Returns the updated user object.
pub(crate) async fn postpone_initial_sync(
    user: RequestUser,
    State(store): State<DbPool>,
) -> ApiResult<tb_domain::User> {
    let mut store = store.begin().await?;

    // Check if user is still pending
    let user_data = user.tb_id().read(&mut store).await?;
    if user_data.onboarding_status != OnboardingStatus::Pending {
        return Err(
            Error::BadRequest("Initial sync already completed or postponed".to_string()).into(),
        );
    }

    // Mark as postponed and return updated user
    let updated_user = store
        .update_onboarding_status(&user.tb_id(), OnboardingStatus::InitialSyncPostponed)
        .await?;

    store.commit().await?;
    Ok(Json(updated_user))
}
