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

use anyhow::ensure;
use async_session::log::{trace, info};
use axum::{Json, extract::{Query, Path, State}};
use {tb_domain::{AnyResult, Error, Summary}};

use tb_strava::{event::{InEvent, process}, StravaUser};
use serde_derive::{ Deserialize, Serialize};

use crate::{ApiResult, RequestUser, AxumAdmin, DbPool};

use super::refresh_token;

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
    fn validate(self) -> AnyResult<Hub> {
        ensure!(
            self.verify_token == VERIFY_TOKEN, 
            Error::BadRequest(format!("Unknown verify token {}", self.verify_token))
        );
        ensure!(
            self.mode == "subscribe", 
            Error::BadRequest(format!("Unknown mode {}", self.mode))
        );
        Ok(self)
    }
}

const VERIFY_TOKEN: &str = "tendabike_strava";

pub(crate) async fn hooks (user: RequestUser, State(conn): State<DbPool>) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    let user = user.get_strava_user(&mut conn).await?;
    user.lock(&mut conn).await?;
    let res = process(&user, &mut conn).await;
    user.unlock(&mut conn).await?;
    Ok(Json(res?))
}

pub(crate) async fn create_event(State(conn): State<DbPool>, Json(event): axum::extract::Json<InEvent>) -> ApiResult<()> {
    trace!("Received {:#?}", event);
    let mut conn = conn.get().await?;
    event.accept(&mut conn).await?;
    Ok(Json(()))
}

pub(super) async fn validate_subscription (Query(hub): Query<Hub>) -> ApiResult<Hub> {
    info!("Received validation callback {:?}", hub);
    Ok(hub.validate().map(Json)?)
}

#[derive(Deserialize)]
pub(super) struct SyncQuery {
    time: i64,
    user_id: Option<i32>,
}

pub(super) async fn sync_api (_u: AxumAdmin, State(conn): State<DbPool>, Query(query): Query<SyncQuery>) -> ApiResult<()> {
    let mut conn = conn.get().await?;
    let user_id: Option<tb_domain::UserId> = query.user_id.map(|u| u.into());
    Ok(tb_strava::sync_users(user_id, query.time, &mut conn).await.map(Json)?)
}


pub(super) async fn sync(Path(tbid): Path<i32>, _u: AxumAdmin, State(conn): State<DbPool>) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    let conn = &mut conn;
    let user = StravaUser::read(tbid.into(), conn).await?;
    let user = refresh_token(user, conn).await?;
    user.lock( conn).await?;
    let res = process(&user, conn).await;
    user.unlock(conn).await?;
    Ok(Json(res?))
}
