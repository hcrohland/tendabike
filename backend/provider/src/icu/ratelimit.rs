//! Proactive intervals.icu rate limiting: a thin shim over the shared
//! [`tb_provider::RateLimitShim`] kit module.
//!
//! The process-wide limiter is constructed with intervals.icu's header
//! mapping — a 15-minute and a daily window, both reading the
//! `X-RateLimit-Limit` / `X-RateLimit-Remaining` pair (the API reports
//! *remaining* requests). The HTTP layer feeds it from response headers
//! ([`update_from_headers`]); the event-queue read consults it for the
//! "limited until" gate ([`limited_until`]).
//!
//! The state is process-wide (shared across all executors) and in-memory: on
//! restart it starts empty and learns from the first response.
//!
//! Test seam: the test hooks set a per-thread override limiter that
//! [`limited_until`] reports from, so parallel rate-limiter tests can't
//! observe each other's process-global state.

use std::cell::RefCell;

use http::{HeaderMap, HeaderName};

use crate::{Cadence, RateLimitShim, RateLimiter, UsageKind, WindowSpec};

/// intervals.icu's 2-window header mapping: a 15-minute and a daily window,
/// both reading the `X-RateLimit-Limit` / `X-RateLimit-Remaining` pair. The
/// API reports *remaining* requests (not used), so the usage is derived as
/// `limit - remaining`. There is no separate read scope.
fn icu_specs() -> Vec<WindowSpec> {
    vec![
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-RateLimit-Remaining").unwrap(),
            cadence: Cadence::FifteenMin,
            usage_kind: UsageKind::Remaining,
        },
        WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-RateLimit-Remaining").unwrap(),
            cadence: Cadence::Daily,
            usage_kind: UsageKind::Remaining,
        },
    ]
}

/// The process-wide icu rate limiter (shared across all executors).
static RATE_LIMITER: std::sync::LazyLock<RateLimitShim> =
    std::sync::LazyLock::new(|| RateLimitShim::new("icu", icu_specs()));

// A per-thread override limiter, set by the test hooks
// (`test_set_limited` / `test_set_not_limited`). When `Some`, `limited_until`
// reports from this limiter instead of the process-global `RATE_LIMITER`, so
// parallel rate-limiter tests can't observe each other's state (each test runs
// on its own thread). `None` in production and on any thread a test hook
// hasn't touched.
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

/// Parses intervals.icu's `X-RateLimit-*` response headers into the shared
/// state. Each header is a `"15min,daily"` comma-separated pair of integers.
/// Missing or malformed headers are ignored (the state keeps its previous
/// values for those fields).
pub fn update_from_headers(headers: &HeaderMap) {
    RATE_LIMITER.update_from_headers(headers);
}

#[cfg(test)]
/// Sets this thread's rate-limiter override to "at limit" (15-min exhausted:
/// 0 remaining), so [`limited_until`] reports a boundary.
pub fn test_set_limited() {
    TEST_OVERRIDE.with(|o| {
        *o.borrow_mut() = Some(RateLimitShim::limiter_from_headers(
            "icu",
            &icu_specs(),
            &[
                ("X-RateLimit-Limit", "100,1000"),
                ("X-RateLimit-Remaining", "0,500"),
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
            "icu",
            &icu_specs(),
            &[
                ("X-RateLimit-Limit", "100,1000"),
                ("X-RateLimit-Remaining", "100,1000"),
            ],
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icu_limited_until_reports_boundary_when_exhausted() {
        test_set_limited();
        let until = limited_until().expect("exhausted read window reports a boundary");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(until > now, "boundary is in the future");
        assert_eq!(until % 900, 0, "boundary is a 15-min mark");
    }

    #[test]
    fn icu_limited_until_is_none_when_not_exhausted() {
        test_set_not_limited();
        assert_eq!(limited_until(), None);
    }

    /// The intervals.icu API returns exactly these two headers (per the API
    /// docs): `X-RateLimit-Limit: <15m>,<daily>` and
    /// `X-RateLimit-Remaining: <15m>,<daily>`. This feeds the real shape and
    /// asserts the limiter detects an exhausted 15-minute window (0 remaining).
    #[test]
    fn icu_limiter_works_with_real_api_headers() {
        let mut m = HeaderMap::new();
        m.insert(
            HeaderName::from_bytes(b"X-RateLimit-Limit").unwrap(),
            http::HeaderValue::from_str("2500,5000").unwrap(),
        );
        m.insert(
            HeaderName::from_bytes(b"X-RateLimit-Remaining").unwrap(),
            http::HeaderValue::from_str("0,4500").unwrap(),
        );
        let mut limiter = RateLimiter::new("icu", icu_specs());
        limiter.update_from_headers(&m);
        let until = limiter
            .gate()
            .expect("exhausted 15-minute window reports a boundary");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(until > now, "boundary is in the future");
        assert_eq!(until % 900, 0, "boundary is a 15-min mark");
    }
}
