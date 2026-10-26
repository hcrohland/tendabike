//! Proactive Strava rate limiting: reads usage and limits from response
//! headers, gates event processing when budget is exhausted.
//!
//! The state is process-wide (shared across all executors) and in-memory:
//! on restart it starts empty and learns from the first response.

use std::sync::Mutex;

use http::HeaderMap;

/// The rate-limit state: usage and limits for both the overall and the
/// non-upload (read) scopes, each with a 15-minute and a daily window.
/// `None` until the first response populates the values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RateLimitState {
    pub overall_limit_15min: Option<u32>,
    pub overall_usage_15min: Option<u32>,
    pub overall_limit_daily: Option<u32>,
    pub overall_usage_daily: Option<u32>,
    pub read_limit_15min: Option<u32>,
    pub read_usage_15min: Option<u32>,
    pub read_limit_daily: Option<u32>,
    pub read_usage_daily: Option<u32>,
    /// Whether the last `limited_until()` call reported a limit (for WARN
    /// log transitions: entering / exiting the rate-limited state).
    pub was_limited: bool,
}

static STATE: std::sync::LazyLock<Mutex<RateLimitState>> =
    std::sync::LazyLock::new(|| Mutex::new(RateLimitState::default()));

/// The shared rate-limit state: the unix time at which the exhausted window
/// resets, or `None` if budget remains (or no data yet).
///
/// Emits a WARN log on transition into or out of the rate-limited state.
pub fn limited_until() -> Option<i64> {
    let mut state = STATE.lock().unwrap();
    let until = limited_until_inner(&state);
    let is_limited = until.is_some();
    if is_limited && !state.was_limited {
        log::warn!(
            "Strava rate limit reached (read 15min: {}/{}, read daily: {}/{}, overall 15min: {}/{}, overall daily: {}/{}); pausing until {}",
            state.read_usage_15min.unwrap_or(0),
            state.read_limit_15min.unwrap_or(0),
            state.read_usage_daily.unwrap_or(0),
            state.read_limit_daily.unwrap_or(0),
            state.overall_usage_15min.unwrap_or(0),
            state.overall_limit_15min.unwrap_or(0),
            state.overall_usage_daily.unwrap_or(0),
            state.overall_limit_daily.unwrap_or(0),
            until.unwrap_or(0),
        );
    } else if !is_limited && state.was_limited {
        log::warn!(
            "Strava rate limit cleared (read 15min: {}/{}, read daily: {}/{})",
            state.read_usage_15min.unwrap_or(0),
            state.read_limit_15min.unwrap_or(0),
            state.read_usage_daily.unwrap_or(0),
            state.read_limit_daily.unwrap_or(0),
        );
    }
    state.was_limited = is_limited;
    until
}

/// Parses Strava's `X-RateLimit-*` response headers into the shared state.
///
/// Each header is a `"15min,daily"` comma-separated pair of integers.
/// Missing or malformed headers are ignored (the state keeps its previous
/// values for those fields).
pub fn update_from_headers(headers: &HeaderMap) {
    let mut state = STATE.lock().unwrap();
    update_from_headers_inner(&mut state, headers);
}

fn limited_until_inner(state: &RateLimitState) -> Option<i64> {
    // The earliest boundary of any exhausted window.
    [
        exhausted(
            state.read_usage_15min,
            state.read_limit_15min,
            next_15min_boundary,
        ),
        exhausted(
            state.read_usage_daily,
            state.read_limit_daily,
            next_midnight_utc,
        ),
        exhausted(
            state.overall_usage_15min,
            state.overall_limit_15min,
            next_15min_boundary,
        ),
        exhausted(
            state.overall_usage_daily,
            state.overall_limit_daily,
            next_midnight_utc,
        ),
    ]
    .into_iter()
    .flatten()
    .min()
}

/// Returns `Some(boundary())` when `usage >= limit`, else `None`.
fn exhausted(usage: Option<u32>, limit: Option<u32>, boundary: fn() -> i64) -> Option<i64> {
    match (usage, limit) {
        (Some(u), Some(l)) if u >= l => Some(boundary()),
        _ => None,
    }
}

fn update_from_headers_inner(state: &mut RateLimitState, headers: &HeaderMap) {
    if let Some(val) = parse_pair(headers.get("X-RateLimit-Limit")) {
        state.overall_limit_15min = Some(val.0);
        state.overall_limit_daily = Some(val.1);
    }
    if let Some(val) = parse_pair(headers.get("X-RateLimit-Usage")) {
        state.overall_usage_15min = Some(val.0);
        state.overall_usage_daily = Some(val.1);
    }
    if let Some(val) = parse_pair(headers.get("X-ReadRateLimit-Limit")) {
        state.read_limit_15min = Some(val.0);
        state.read_limit_daily = Some(val.1);
    }
    if let Some(val) = parse_pair(headers.get("X-ReadRateLimit-Usage")) {
        state.read_usage_15min = Some(val.0);
        state.read_usage_daily = Some(val.1);
    }
}

/// Parses a `"15min,daily"` comma-separated header value into a pair.
fn parse_pair(header: Option<&http::HeaderValue>) -> Option<(u32, u32)> {
    let s = header?.to_str().ok()?;
    let mut parts = s.split(',');
    let first = parts.next()?.trim().parse::<u32>().ok()?;
    let second = parts.next()?.trim().parse::<u32>().ok()?;
    Some((first, second))
}

/// The current unix time in seconds.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The next 15-minute window boundary (:00, :15, :30, :45) in unix seconds.
fn next_15min_boundary() -> i64 {
    (now_unix() / 900 + 1) * 900
}

/// The next midnight UTC in unix seconds.
fn next_midnight_utc() -> i64 {
    (now_unix() / 86400 + 1) * 86400
}

#[cfg(test)]
pub fn test_set_limited() {
    let mut state = STATE.lock().unwrap();
    state.read_usage_15min = Some(100);
    state.read_limit_15min = Some(100);
    state.read_usage_daily = Some(500);
    state.read_limit_daily = Some(1000);
}

#[cfg(test)]
pub fn test_set_not_limited() {
    let mut state = STATE.lock().unwrap();
    state.read_usage_15min = Some(0);
    state.read_limit_15min = Some(100);
    state.read_usage_daily = Some(0);
    state.read_limit_daily = Some(1000);
}

#[cfg(test)]
mod ratelimit_tests;
