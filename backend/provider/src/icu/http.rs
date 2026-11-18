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

use super::{IcuDataStore, IcuId};
use crate::facade::{Error, TbResult, UserId};
use crate::{StatusDecision, classify_status};

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

    // The offline seam (test-only): the kit's thread-local canned response
    // (per-test isolation: each test runs on its own thread). A test-support
    // build has no real intervals.icu API: an unscripted URI panics rather
    // than pretend.
    #[cfg(any(test, feature = "test-support"))]
    if let Some(body) = crate::test_support::pop_test_canned(uri, "icu") {
        let body = body?;
        return Ok(serde_json::from_str(&body).context("Could not parse response body")?);
    }

    let r = crate::http::bearer_get(
        API,
        &access,
        uri,
        &super::ratelimit::update_from_headers,
        &IcuMapping,
    )
    .await?;
    trace!("{uri}:\n{r}");
    Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
}

/// intervals.icu's status-to-error mapping: the kit's status classification
/// applied with icu's message wording. The `Permanent` arm reads the response
/// body into the error detail (icu-specific).
struct IcuMapping;

impl crate::http::BearerGetMapping for IcuMapping {
    fn reads_body(&self) -> bool {
        true
    }
    fn map(&self, status: StatusCode, uri: &str, body: Option<String>) -> Error {
        match classify_status(status) {
            StatusDecision::TryAgain => Error::TryAgain(status.canonical_reason().unwrap()),
            StatusDecision::NotAuth => Error::NotAuth(format!(
                "intervals.icu request authorization withdrawn for {uri}"
            )),
            StatusDecision::Permanent => Error::BadRequest(format!(
                "intervals.icu request error: {} — {}",
                status
                    .canonical_reason()
                    .unwrap_or("Unknown status received"),
                body.unwrap_or_default()
            )),
        }
    }
}

#[cfg(test)]
mod http_tests;
