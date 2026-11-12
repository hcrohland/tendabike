//! Proactive Strava rate limiting: a thin shim over the shared
//! [`tb_provider::RateLimitShim`] kit module.
//!
//! The process-wide limiter is constructed with Strava's 4-window header
//! mapping — the overall and read scopes, each a 15-minute and a daily window
//! reading the same `X-RateLimit-*` / `X-ReadRateLimit-*` pair. The HTTP layer
//! feeds it from response headers ([`update_from_headers`]); the event-queue
//! read consults it for the "limited until" gate ([`limited_until`]).
//!
//! The state is process-wide (shared across all executors) and in-memory:
//! on restart it starts empty and learns from the first response.
//!
//! Test seam: the test hooks set a per-thread override limiter that
//! [`limited_until`] reports from, so parallel rate-limiter tests can't
//! observe each other's process-global state.

use std::cell::RefCell;

use http::{HeaderMap, HeaderName};

use crate::{Cadence, RateLimitShim, RateLimiter, WindowSpec};

/// Strava's 4-window header mapping: the overall and read scopes, each a
/// 15-minute and a daily window reading the same header pair.
fn strava_specs() -> Vec<WindowSpec> {
    vec![
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-RateLimit-Usage").unwrap(),
            cadence: Cadence::FifteenMin,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-RateLimit-Usage").unwrap(),
            cadence: Cadence::Daily,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-ReadRateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-ReadRateLimit-Usage").unwrap(),
            cadence: Cadence::FifteenMin,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-ReadRateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-ReadRateLimit-Usage").unwrap(),
            cadence: Cadence::Daily,
        },
    ]
}

/// The process-wide Strava rate limiter (shared across all executors).
static RATE_LIMITER: std::sync::LazyLock<RateLimitShim> =
    std::sync::LazyLock::new(|| RateLimitShim::new("strava", strava_specs()));

// A per-thread override limiter, set by the test hooks
// (`test_set_limited` / `test_set_not_limited`). When `Some`, `limited_until`
// reports from this limiter instead of the process-global `RATE_LIMITER`, so
// parallel rate-limiter tests can't observe each other's state (each test runs
// on its own thread, the same isolation the `test_support::CANNED`
// canned-response map relies on). `None` in production and on any thread a
// test hook hasn't touched.
thread_local! {
    static TEST_OVERRIDE: RefCell<Option<RateLimiter>> = const { RefCell::new(None) };
}

/// The shared rate-limit gate: the unix time at which the exhausted window
/// resets, or `None` if budget remains (or no data yet).
///
/// Emits a WARN log on transition into or out of the rate-limited state.
pub fn limited_until() -> Option<i64> {
    // Test seam: a per-thread override shadows the process-global limiter, so
    // parallel rate-limiter tests can't observe each other's state.
    if let Some(until) = TEST_OVERRIDE.with(|o| o.borrow().as_ref().map(|l| l.gate())) {
        return until;
    }
    RATE_LIMITER.limited_until()
}

/// Parses Strava's `X-RateLimit-*` response headers into the shared state.
///
/// Each header is a `"15min,daily"` comma-separated pair of integers.
/// Missing or malformed headers are ignored (the state keeps its previous
/// values for those fields).
pub fn update_from_headers(headers: &HeaderMap) {
    RATE_LIMITER.update_from_headers(headers);
}

#[cfg(test)]
/// Sets this thread's rate-limiter override to "at limit" (read 15-min
/// exhausted), so [`limited_until`] reports a boundary.
pub fn test_set_limited() {
    TEST_OVERRIDE.with(|o| {
        *o.borrow_mut() = Some(RateLimitShim::limiter_from_headers(
            "strava",
            &strava_specs(),
            &[
                ("X-ReadRateLimit-Limit", "100,1000"),
                ("X-ReadRateLimit-Usage", "100,500"),
            ],
        ));
    });
}

#[cfg(test)]
/// Sets this thread's rate-limiter override to "no data" (allows all
/// requests), so [`limited_until`] reports no limit.
pub fn test_set_not_limited() {
    TEST_OVERRIDE.with(|o| {
        *o.borrow_mut() = Some(RateLimitShim::limiter_from_headers(
            "strava",
            &strava_specs(),
            &[
                ("X-ReadRateLimit-Limit", "100,1000"),
                ("X-ReadRateLimit-Usage", "0,0"),
            ],
        ));
    });
}
