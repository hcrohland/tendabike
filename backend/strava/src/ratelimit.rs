//! Proactive Strava rate limiting: reads usage and limits from response
//! headers, gates event processing when budget is exhausted.
//!
//! The state is process-wide (shared across all executors) and in-memory:
//! on restart it starts empty and learns from the first response.

use std::sync::Mutex;

use http::HeaderMap;

/// A single rate-limit window: the limit and the current usage.
/// `None` until the first response populates the values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Window {
    pub limit: Option<u32>,
    pub usage: Option<u32>,
}

impl Window {
    /// Whether the budget is exhausted (usage >= limit).
    /// `None` values (no data yet) are not exhausted.
    pub fn is_exhausted(&self) -> bool {
        match (self.usage, self.limit) {
            (Some(u), Some(l)) => u >= l,
            _ => false,
        }
    }
}

impl std::fmt::Display for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.usage.unwrap_or(0), self.limit.unwrap_or(0))
    }
}

/// The rate-limit state: usage and limits for both the overall and the
/// non-upload (read) scopes, each with a 15-minute and a daily window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RateLimitState {
    pub overall_15min: Window,
    pub overall_daily: Window,
    pub read_15min: Window,
    pub read_daily: Window,
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
            "Strava rate limit reached (read 15min: {}, read daily: {}, overall 15min: {}, overall daily: {}); pausing until {}",
            state.read_15min,
            state.read_daily,
            state.overall_15min,
            state.overall_daily,
            until.unwrap_or(0),
        );
    } else if !is_limited && state.was_limited {
        log::warn!(
            "Strava rate limit cleared (read 15min: {}, read daily: {})",
            state.read_15min,
            state.read_daily,
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
    // The latest boundary of any exhausted window: the longest limit wins,
    // so we sleep until the last window resets (waking earlier would find
    // the other window still exhausted).
    [
        state.read_15min.is_exhausted().then(next_15min_boundary),
        state.read_daily.is_exhausted().then(next_midnight_utc),
        state.overall_15min.is_exhausted().then(next_15min_boundary),
        state.overall_daily.is_exhausted().then(next_midnight_utc),
    ]
    .into_iter()
    .flatten()
    .max()
}

fn update_from_headers_inner(state: &mut RateLimitState, headers: &HeaderMap) {
    if let Some(val) = parse_pair(headers.get("X-RateLimit-Limit")) {
        state.overall_15min.limit = Some(val.0);
        state.overall_daily.limit = Some(val.1);
    }
    if let Some(val) = parse_pair(headers.get("X-RateLimit-Usage")) {
        state.overall_15min.usage = Some(val.0);
        state.overall_daily.usage = Some(val.1);
    }
    if let Some(val) = parse_pair(headers.get("X-ReadRateLimit-Limit")) {
        state.read_15min.limit = Some(val.0);
        state.read_daily.limit = Some(val.1);
    }
    if let Some(val) = parse_pair(headers.get("X-ReadRateLimit-Usage")) {
        state.read_15min.usage = Some(val.0);
        state.read_daily.usage = Some(val.1);
    }
    log::debug!(
        "Strava rate limit state updated: read 15min: {}, read daily: {}, overall 15min: {}, overall daily: {}",
        state.read_15min,
        state.read_daily,
        state.overall_15min,
        state.overall_daily,
    );
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
    state.read_15min = Window {
        limit: Some(100),
        usage: Some(100),
    };
    state.read_daily = Window {
        limit: Some(1000),
        usage: Some(500),
    };
}

#[cfg(test)]
pub fn test_set_not_limited() {
    let mut state = STATE.lock().unwrap();
    state.read_15min = Window {
        limit: Some(100),
        usage: Some(0),
    };
    state.read_daily = Window {
        limit: Some(1000),
        usage: Some(0),
    };
}

#[cfg(test)]
mod ratelimit_tests;
