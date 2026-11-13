//! HTTP status classification for provider API calls, owned by the provider
//! kit: the kit classifies a non-success status into a decision — a pure
//! function, testable without a client — and each provider module applies
//! the decision to its own `Error` variant and message.

use http::StatusCode;

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
