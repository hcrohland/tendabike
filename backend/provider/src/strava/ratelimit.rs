//! Proactive Strava rate limiting: a thin shim over the shared
//! [`tb_provider::RateLimitHandle`] kit module.
//!
//! The process-wide limiter is the kit's [`RateLimitHandle`], constructed with
//! Strava's 4-window header mapping — the only genuine provider difference.
//! The HTTP layer feeds it from response headers ([`update_from_headers`]);
//! the event-queue read consults it for the "limited until" gate
//! ([`limited_until`]).
//!
//! The state is process-wide (shared across all executors) and in-memory: on
//! restart it starts empty and learns from the first response.
//!
//! Test seam: the test hook ([`test_set_not_limited`]) drives the kit's
//! single shared per-thread override thread-local, so parallel rate-limiter
//! tests can't observe each other's process-global state.

use http::{HeaderMap, HeaderName};

use crate::{Cadence, RateLimitHandle, UsageKind, WindowSpec};

/// Strava's 4-window header mapping: the overall and read scopes, each a
/// 15-minute and a daily window reading the same header pair. Strava reports
/// *used* requests (`X-RateLimit-Usage`).
fn strava_specs() -> Vec<WindowSpec> {
    vec![
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-RateLimit-Usage").unwrap(),
            cadence: Cadence::FifteenMin,
            usage_kind: UsageKind::Used,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-RateLimit-Usage").unwrap(),
            cadence: Cadence::Daily,
            usage_kind: UsageKind::Used,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-ReadRateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-ReadRateLimit-Usage").unwrap(),
            cadence: Cadence::FifteenMin,
            usage_kind: UsageKind::Used,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-ReadRateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-ReadRateLimit-Usage").unwrap(),
            cadence: Cadence::Daily,
            usage_kind: UsageKind::Used,
        },
    ]
}

/// The process-wide Strava rate limiter (shared across all executors).
static RATE_LIMITER: std::sync::LazyLock<RateLimitHandle> =
    std::sync::LazyLock::new(|| RateLimitHandle::new("strava", strava_specs()));

/// The shared rate-limit gate: the unix time at which the exhausted window
/// resets, or `None` if budget remains (or no data yet).
///
/// Emits a WARN log on transition into or out of the rate-limited state.
/// `pub` (not `pub(crate)`) so the
/// [`Store`](crate::store::Store) impl over the data store can consult the
/// gate (the queue's read-side rate-limit check, spec #1029).
pub fn limited_until() -> Option<i64> {
    RATE_LIMITER.limited_until()
}

/// Parses Strava's `X-RateLimit-*` response headers into the shared state.
///
/// Each header is a `"15min,daily"` comma-separated pair of integers.
/// Missing or malformed headers are ignored (the state keeps its previous
/// values for those fields).
pub(crate) fn update_from_headers(headers: &HeaderMap) {
    RATE_LIMITER.update_from_headers(headers);
}

#[cfg(test)]
/// Sets this thread's rate-limiter override to "no data" (allows all
/// requests), so [`limited_until`] reports no limit.
pub(crate) fn test_set_not_limited() {
    RATE_LIMITER.test_set_not_limited();
}
