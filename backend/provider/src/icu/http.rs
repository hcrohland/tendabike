//! The thin intervals.icu HTTP calls: [`request_json`].
//!
//! Unlike the Strava driver there is no refresh machinery — intervals.icu
//! tokens are durable, so [`access_token`] reads the (durable) access token
//! straight from `icu_users` and [`request_json`] makes the one bearer GET.
//! A `401` (a revoked token) is `Error::NotAuth` (the caller disables the
//! user); a `429` / `5xx` is `Error::TryAgain` (the rate-limit state is
//! already updated from the response headers, so the queue's probe reports
//! the window boundary and the loop sleeps until it).

use anyhow::Context;
use http::StatusCode;
use log::trace;
use serde::de::DeserializeOwned;

use super::client::HTTP_CLIENT;
use super::{IcuDataStore, IcuId};
use tb_domain::{Error, TbResult, UserId};

const API: &str = "https://intervals.icu/api/v1";

/// A valid access token for `user_id`: the (durable) token stored in
/// `icu_users`. No refresh — intervals.icu tokens never rotate.
async fn access_token(user_id: UserId, store: &mut impl IcuDataStore) -> TbResult<(IcuId, String)> {
    let user = store.icuser_get_by_tbid(user_id).await?;
    let token = user
        .access_token
        .clone()
        .ok_or_else(|| Error::NotAuth(format!("no icu access token for user {user_id}")))?;
    Ok((user.id, token))
}

/// A bearer GET of `uri` for `user`, parsed as `T`.
///
/// `access_token` → bearer GET + parse.
///
/// `access_token` runs before the offline seams so a token read is exercised
/// even when a canned response stands in for the API. In a `test-support`
/// build the seam below exhausts every URI (canned or panic), so the bearer
/// GET is unreachable — expected there, hence the targeted allow.
#[cfg_attr(feature = "test-support", allow(unused_variables, unreachable_code))]
pub(crate) async fn request_json<T: DeserializeOwned>(
    user_id: UserId,
    uri: &str,
    store: &mut impl IcuDataStore,
) -> TbResult<T> {
    let (_icuid, access) = access_token(user_id, store).await?;

    // The offline seam (test-only): the thread-local canned response
    // (per-test isolation: each test runs on its own thread). A test-support
    // build has no real intervals.icu API: an unscripted URI panics rather
    // than pretend.
    #[cfg(any(test, feature = "test-support"))]
    if let Some(body) = super::test_support::pop_test_canned(uri) {
        let body = body?;
        return Ok(serde_json::from_str(&body).context("Could not parse response body")?);
    }

    let r = get_icu(&access, uri).await?;
    trace!("{uri}:\n{r}");
    Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
}

/// The bearer GET of an intervals.icu API `uri`, mapping the HTTP status to a
/// domain error (the same classification the Strava driver's `get_strava`
/// uses).
async fn get_icu(access: &str, uri: &str) -> TbResult<String> {
    let resp = HTTP_CLIENT
        .get(format!("{API}{uri}"))
        .bearer_auth(access)
        .send()
        .await
        .context("Could not reach intervals.icu")?;

    // Update the rate-limit state from the response headers: every intervals.icu
    // API response carries X-RateLimit-* headers (success or error).
    super::ratelimit::update_from_headers(resp.headers());

    let status = resp.status();
    if status.is_success() {
        return Ok(resp.text().await.context("could not reach intervals.icu")?);
    }

    match status {
        StatusCode::TOO_MANY_REQUESTS
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => Err(Error::TryAgain(status.canonical_reason().unwrap())),
        StatusCode::UNAUTHORIZED => Err(Error::NotAuth(format!(
            "intervals.icu request authorization withdrawn for {uri}"
        ))),
        _ => Err(Error::BadRequest(format!(
            "intervals.icu request error: {}",
            status
                .canonical_reason()
                .unwrap_or("Unknown status received")
        ))),
    }
}

#[cfg(test)]
mod http_tests;
