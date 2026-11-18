//! HTTP status classification and the shared bearer-GET shape for provider
//! API calls, owned by the provider kit: the kit classifies a non-success
//! status into a decision — a pure function, testable without a client — and
//! each provider module applies the decision to its own `Error` variant and
//! message through the [`BearerGetMapping`] hook. The [`bearer_get`] shape
//! owns the one HTTP call both providers make (a bearer GET of the provider's
//! base URL), over the kit's shared no-redirect client.

use anyhow::Context;
use http::{HeaderMap, StatusCode};

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
/// Pure: no I/O. Each provider maps the decision onto its own `Error`
/// variant with its own message (and, for the icu driver, reads the response
/// body into the `Permanent` arm — provider-specific, so it stays there).
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

/// The provider's status-to-error mapping: the kit's [`classify_status`]
/// applied with the provider's message wording, and (for the `Permanent` arm)
/// an optional body-reading extension. A driver supplies its own impl — the
/// kit's [`bearer_get`] shape applies it to a non-success response. `Sync` so
/// the shape's future stays `Send` (the executor loop drives it across
/// threads).
pub(crate) trait BearerGetMapping: Sync {
    /// Map a non-success status to a domain error, with the provider's
    /// message wording. `body` is the response body when the provider reads
    /// it in the `Permanent` arm (intervals.icu does; Strava does not, and
    /// receives `None`).
    fn map(&self, status: StatusCode, uri: &str, body: Option<String>) -> Error;

    /// Whether the `Permanent` arm reads the response body into the error
    /// detail (intervals.icu: true; Strava: false).
    fn reads_body(&self) -> bool;
}

/// The shared bearer-GET shape: a bearer GET of `{base}{uri}` over the kit's
/// shared no-redirect client (following redirects opens the client to SSRF),
/// updating the provider's rate-limit state from the response headers, and
/// mapping a non-success status through `mapping`.
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
    mapping: &dyn BearerGetMapping,
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

    let body = if mapping.reads_body() {
        Some(resp.text().await.unwrap_or_default())
    } else {
        None
    };
    Err(mapping.map(status, uri, body))
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
