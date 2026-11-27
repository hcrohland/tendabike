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

//! The thin Strava HTTP calls: [`request_json`] and [`access_token`].
//!
//! [`access_token`] owns the one piece of refresh machinery the HTTP calls
//! share: it calls the `TokenCache`'s `get_token` and persists the refresh
//! token to the DB when a refresh issued a new one. `request_json` then
//! resolves the token, handles Strava's own extra offline seam (the
//! process-wide canned-response global), and makes the one HTTP call through
//! the kit's shared request — it knows nothing about refresh mechanics.

#[cfg(feature = "test-support")]
use anyhow::Context;
use oauth2::AccessToken;
use serde::de::DeserializeOwned;

use super::session::get_token;
use super::{StravaId, StravaSpec};
use crate::facade::{TbResult, UserId};
use crate::store::Store;

const API: &str = "https://www.strava.com/api/v3";

/// A valid access token for `user_id`: the `TokenCache`'s `get_token`, which
/// owns the refresh exchange and persists the rotated refresh token to the
/// DB when a refresh issues a new one.
pub(crate) async fn access_token(
    user_id: UserId,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<(StravaId, AccessToken)> {
    get_token(user_id, store).await
}

/// A bearer GET of `uri` for `user`, parsed as `T`: resolve the token
/// (refresh vs durable), Strava's own extra offline seam, then the kit's
/// shared request (the shared per-thread offline seam, the bearer GET, the
/// parse). Knows nothing about refresh mechanics.
///
/// `get_token` runs before the offline seams so a rotated refresh token is
/// persisted even when a canned response stands in for the API.
pub(crate) async fn request_json<T: DeserializeOwned>(
    user_id: UserId,
    uri: &str,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<T> {
    let (_strava_id, access) = access_token(user_id, store).await?;

    // Strava's extra offline seam (test-only): the process-wide
    // `CANNED_STRAVA` global (the seam suite's session, run in a spawned task
    // on a thread the test-local map never reached). A test-support build has
    // no real Strava API: an unscripted URI panics rather than pretend (the
    // seam suite's stubs never import from Strava; the onboarding test
    // expects the loop to panic on the sync's fetch).
    #[cfg(feature = "test-support")]
    match super::test_support::global_canned_response(uri) {
        Some(json) => {
            return Ok(serde_json::from_str(&json).context("Could not parse response body")?);
        }
        // In test mode the kit's shared per-thread canned-response seam is
        // the next stop; the seam suite's threads are never in test mode, so
        // an unscripted URI panics there.
        None if crate::test_support::in_test_mode() => {}
        None => panic!("unexpected strava request: {uri}"),
    }

    crate::http::request_json::<T>(
        API,
        &access,
        uri,
        &super::ratelimit::update_from_headers,
        "Strava",
    )
    .await
}

#[cfg(test)]
mod http_tests;
