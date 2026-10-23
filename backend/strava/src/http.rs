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

//! The thin Strava HTTP calls: [`request_json`] and [`deauthorize`].
//!
//! These free functions are the Strava API surface. They know nothing about
//! refresh mechanics: each calls the session's
//! [`get_token`](crate::StravaSession::get_token), persists the refresh token
//! to the DB when a refresh issued a new one, and then makes the one HTTP call
//! (the bearer GET, or the token revocation).

use anyhow::Context;
use http::StatusCode;
use log::{debug, trace};
use oauth2::{AccessToken, RefreshToken};
use serde::de::DeserializeOwned;

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::{StravaDataStore, StravaSession};
use tb_domain::{Error, TbResult};

const API: &str = "https://www.strava.com/api/v3";

/// A bearer GET of `uri` for `user`, parsed as `T`.
///
/// `get_token` → persist the refresh token to the DB if a refresh issued a new
/// one → bearer GET + parse. Knows nothing about refresh mechanics.
///
/// `get_token` runs before the offline seams so a rotated refresh token is
/// persisted even when a canned response stands in for the API. In a
/// `test-support` build the seams below exhaust every URI (canned or panic),
/// so the bearer GET is unreachable and its `access` goes unused — expected
/// there, hence the targeted allow.
#[cfg_attr(feature = "test-support", allow(unused_variables, unreachable_code))]
pub async fn request_json<T: DeserializeOwned>(
    user: &mut dyn StravaSession,
    uri: &str,
    store: &mut impl StravaDataStore,
) -> TbResult<T> {
    let (access, new_refresh) = user.get_token().await?;
    persist_refresh(user, store, new_refresh).await?;

    // The offline seams (test-only): first the thread-local canned response
    // (per-test isolation: each test runs on its own thread), then the
    // process-wide `CANNED_STRAVA` global (the seam suite's session, run in a
    // spawned task on a thread the test-local map never reached). A
    // test-support build has no real Strava API: an unscripted URI panics
    // rather than pretend (the seam suite's stubs never import from Strava;
    // the onboarding test expects the loop to panic on the sync's fetch).
    #[cfg(any(test, feature = "test-support"))]
    if let Some(body) = crate::test_support::pop_canned(uri) {
        let body = body?;
        return Ok(serde_json::from_str(&body).context("Could not parse response body")?);
    }
    #[cfg(feature = "test-support")]
    match crate::test_support::canned_response(uri) {
        Some(json) => {
            return Ok(serde_json::from_str(&json).context("Could not parse response body")?);
        }
        None => panic!("unexpected strava request: {uri}"),
    }

    let r = get_strava(&access, uri).await?;
    trace!("{uri}:\n{r}");
    Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
}

/// Revoke `user`'s Strava token.
///
/// `get_token` → persist the refresh token to the DB if a refresh issued a new
/// one → `STRAVACLIENT.revoke_token`. Knows nothing about refresh mechanics.
pub async fn deauthorize(
    user: &mut dyn StravaSession,
    store: &mut impl StravaDataStore,
) -> TbResult<()> {
    let (access, new_refresh) = user.get_token().await?;
    persist_refresh(user, store, new_refresh).await?;

    // The offline seam (test-only): in test mode a deauthorization is recorded
    // in the thread-local map instead of hitting the Strava API. The process-
    // wide `CANNED_STRAVA` global does not cover the revocation — the seam
    // suite's real session attempts the real (non-fatal) revocation.
    #[cfg(any(test, feature = "test-support"))]
    if crate::test_support::in_test_mode() {
        crate::test_support::record_deauthorize(user.strava_id());
        debug!("user {} token revoked (test)", user.strava_id());
        return Ok(());
    }

    STRAVACLIENT
        .revoke_token(access.clone().into())
        .context("revoke token config error")?
        .add_extra_param("access_token", access.secret()) // Strava does not follow the standard.
        .request_async(&*HTTP_CLIENT)
        .await
        .context("token exchange failed")?;
    debug!("user {} token revoked", user.strava_id());
    Ok(())
}

/// Persist a freshly-rotated refresh token to the DB. `None` is a no-op: a warm
/// session issued no new token, so there is nothing to persist.
async fn persist_refresh(
    user: &dyn StravaSession,
    store: &mut impl StravaDataStore,
    new_refresh: Option<RefreshToken>,
) -> TbResult<()> {
    if let Some(refresh) = new_refresh {
        let secret = refresh.secret().to_string();
        user.strava_id().update_token(Some(&secret), store).await?;
    }
    Ok(())
}

/// The bearer GET of a Strava API `uri`, mapping the HTTP status to a domain
/// error (the same classification the session's old `get_strava` used).
async fn get_strava(access: &AccessToken, uri: &str) -> TbResult<String> {
    let resp = reqwest::Client::new()
        .get(format!("{API}{uri}"))
        .bearer_auth(access.secret())
        .send()
        .await
        .context("Could not reach strava")?;

    let status = resp.status();
    if status.is_success() {
        return Ok(resp.text().await.context("could not reach strava")?);
    }

    match status {
        StatusCode::TOO_MANY_REQUESTS
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => Err(Error::TryAgain(status.canonical_reason().unwrap())),
        StatusCode::UNAUTHORIZED => Err(Error::NotAuth(format!(
            "Strava request authorization withdrawn for {uri}"
        ))),
        _ => Err(Error::BadRequest(format!(
            "Strava request error: {}",
            status
                .canonical_reason()
                .unwrap_or("Unknown status received")
        ))),
    }
}

#[cfg(test)]
mod http_tests;
