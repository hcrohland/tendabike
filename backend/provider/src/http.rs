//! HTTP status classification and the shared request shape for provider API
//! calls, owned by the provider kit: the kit classifies a non-success status
//! into a decision — a pure function, testable without a client — and
//! [`bearer_get`] applies the decision to the provider's `Error` variant and
//! message. [`request_json`] is the shared provider request both drivers
//! make: the shared per-thread canned-response seam (test-only), the one
//! bearer GET of the provider's base URL over the kit's shared no-redirect
//! client, and the JSON parse.

use anyhow::Context;
use http::{HeaderMap, StatusCode};
use log::trace;
use oauth2::AccessToken;
use serde::de::DeserializeOwned;

use crate::client::HTTP_CLIENT;
use crate::facade::{Error, TbResult};

/// The classification of a non-success provider API response status:
///
/// - a rate limit or transient 5xx (`TryAgain`) → the caller retries after
///   the backoff.
/// - dead auth (`NotAuth`, a 401) → the caller disables the user's
///   integration, which drops all their queued events.
/// - anything else (`Permanent`) → a permanent error: the caller drops the
///   event and moves on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusDecision {
    /// A transient failure (a 429, 502, 503, or 504): the caller retries
    /// after the backoff.
    TryAgain,
    /// Dead auth (a 401): the caller disables the user's integration.
    NotAuth,
    /// Anything else (a 404, a 400, ...): a permanent error.
    Permanent,
}

/// Classifies a non-success provider API response status.
///
/// Pure: no I/O. [`bearer_get`] maps the decision onto the provider's
/// `Error` variant with its own message.
pub fn classify_status(status: StatusCode) -> StatusDecision {
    match status {
        StatusCode::TOO_MANY_REQUESTS
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => StatusDecision::TryAgain,
        StatusCode::UNAUTHORIZED => StatusDecision::NotAuth,
        _ => StatusDecision::Permanent,
    }
}

/// The shared provider request: a bearer GET of `{base}{uri}` parsed as
/// `T`, with the kit's shared per-thread canned-response seam (test-only) in
/// front of the one [`bearer_get`] call.
///
/// `token` is the provider's typed access token (the secret is read here, so
/// no bare-string token crosses the driver seam); `on_headers` is the
/// provider's rate-limit header update (a `fn(&HeaderMap)`), so the shared
/// shape feeds the provider's own limiter; `provider` is the driver's label
/// (the `Error` messages and the seam's "unexpected request" error). The
/// `Sync` bound keeps the shape's future `Send` (the executor loop drives it
/// across threads).
///
/// The offline seam (test-only): the kit's thread-local canned response
/// (per-test isolation: each test runs on its own thread). A driver keeps
/// only its own extra offline seam (Strava's process-wide canned-response
/// global) in its thin wrapper, ahead of this call.
pub(crate) async fn request_json<T: DeserializeOwned>(
    base: &str,
    token: &AccessToken,
    uri: &str,
    on_headers: impl Fn(&HeaderMap) + Sync,
    provider: &str,
) -> TbResult<T> {
    // The shared offline seam (test-only): the kit's thread-local canned
    // response (per-test isolation: each test runs on its own thread). A
    // test-support build has no real provider API: an unscripted URI yields
    // the seam's "unexpected request" error rather than pretend.
    #[cfg(any(test, feature = "test-support"))]
    if let Some(body) = crate::test_support::pop_test_canned(uri, provider) {
        let body = body?;
        return Ok(serde_json::from_str(&body).context("Could not parse response body")?);
    }

    // The real-HTTP path: the one bearer GET, context-wrapped for the
    // provider (the offline seam above returns before this, so a canned
    // error — a `NotAuth`, a `TryAgain` — propagates typed, unwrapped).
    let r = bearer_get(base, token.secret(), uri, on_headers, provider)
        .await
        .with_context(|| format!("Could not reach {provider}"))?;
    trace!("{uri}:\n{r}");
    Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
}

/// The shared bearer-GET shape: a bearer GET of `{base}{uri}` over the kit's
/// shared no-redirect client (following redirects opens the client to SSRF),
/// updating the provider's rate-limit state from the response headers, and
/// mapping a non-success status to the provider's `Error` variant.
///
/// `on_headers` is the provider's rate-limit header update (a
/// `fn(&HeaderMap)`), so the shared shape feeds the provider's own limiter.
/// The `Sync` bound keeps the shape's future `Send` (the executor loop drives
/// it across threads).
pub(crate) async fn bearer_get(
    base: &str,
    token: &str,
    uri: &str,
    on_headers: impl Fn(&HeaderMap) + Sync,
    provider: &str,
) -> TbResult<String> {
    let resp = HTTP_CLIENT
        .get(format!("{base}{uri}"))
        .bearer_auth(token)
        .send()
        .await
        .context("Could not reach the provider API")?;

    on_headers(resp.headers());

    let status = resp.status();
    if status.is_success() {
        return Ok(resp
            .text()
            .await
            .context("could not read the response body")?);
    }

    let body = resp.text().await.unwrap_or_default();
    Err(match classify_status(status) {
        StatusDecision::TryAgain => Error::TryAgain(status.canonical_reason().unwrap()),
        StatusDecision::NotAuth => Error::NotAuth(format!(
            "{provider} request authorization withdrawn for {uri}"
        )),
        StatusDecision::Permanent => Error::BadRequest(format!(
            "{provider} request error: {} — {}",
            status
                .canonical_reason()
                .unwrap_or("Unknown status received"),
            body
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_and_transient_5xx_is_try_again() {
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            assert_eq!(classify_status(status), StatusDecision::TryAgain);
        }
    }

    #[test]
    fn unauthorized_is_not_auth() {
        assert_eq!(
            classify_status(StatusCode::UNAUTHORIZED),
            StatusDecision::NotAuth
        );
    }

    #[test]
    fn everything_else_is_permanent() {
        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            assert_eq!(classify_status(status), StatusDecision::Permanent);
        }
    }
}
