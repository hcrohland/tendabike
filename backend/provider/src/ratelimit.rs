//! Provider-agnostic rate-limit kernel and shared shim: the window model,
//! header-pair parsing, boundary math, and the "latest exhausted window"
//! gate, plus the [`RateLimitShim`] scaffolding a provider module wraps its
//! process-wide limiter in.
//!
//! A provider keeps a `static` [`RateLimitShim`] constructed with its own
//! label and [`WindowSpec`]s (header names + cadence), getting an
//! independent, process-wide limiter. The state is in-memory: on restart it
//! starts empty and learns from the first response.

use std::sync::Mutex;

use http::{HeaderMap, HeaderName, HeaderValue};

/// A single rate-limit window: the limit and the current usage.
/// `None` until the first response populates the values.
#[derive(Debug, Clone, Default, PartialEq)]
struct Window {
    limit: Option<u32>,
    usage: Option<u32>,
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

/// The cadence of a rate-limit window: determines which boundary function
/// to use and which position in a `"15min,daily"` header pair to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    FifteenMin,
    Daily,
}

impl Cadence {
    /// The next boundary for this cadence in unix seconds.
    pub fn boundary(&self) -> i64 {
        match self {
            Cadence::FifteenMin => next_15min_boundary(),
            Cadence::Daily => next_midnight_utc(),
        }
    }

    /// Extract the value for this cadence from a `("15min", "daily")` pair.
    fn pair_value(&self, pair: (u32, u32)) -> u32 {
        match self {
            Cadence::FifteenMin => pair.0,
            Cadence::Daily => pair.1,
        }
    }
}

/// How the usage header's integer is to be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    /// The header reports the number of requests *used* (e.g. Strava's
    /// `X-RateLimit-Usage`). The value is stored as the window's usage.
    Used,
    /// The header reports the number of requests *remaining* (e.g. icu's
    /// `X-RateLimit-Remaining`). The window's usage is derived as
    /// `limit - remaining` (saturating), so `remaining == 0` reads as exhausted.
    Remaining,
}

/// Ties one rate-limit window to its response headers and cadence.
#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub limit_header: HeaderName,
    pub usage_header: HeaderName,
    pub cadence: Cadence,
    pub usage_kind: UsageKind,
}

/// A per-provider rate limiter: one [`Window`] of state per [`WindowSpec`],
/// plus the `was_limited` transition tracker.
pub struct RateLimiter {
    label: &'static str,
    windows: Vec<Window>,
    specs: Vec<WindowSpec>,
    was_limited: bool,
    /// Whether the previous [`update_from_headers`] saw a missing header, so
    /// the missing-header WARN fires on transition (not on every response).
    was_missing: bool,
}

impl RateLimiter {
    /// Create a new limiter with one window per spec.
    ///
    /// `label` identifies the provider in WARN logs (e.g. `"strava"`).
    pub fn new(label: &'static str, specs: Vec<WindowSpec>) -> Self {
        let windows = vec![Window::default(); specs.len()];
        Self {
            label,
            windows,
            specs,
            was_limited: false,
            was_missing: false,
        }
    }

    /// The current window states, in spec order.
    #[cfg(test)]
    fn windows(&self) -> &[Window] {
        &self.windows
    }

    /// Update the window states from response headers.
    ///
    /// For each spec, parses the limit and usage headers (a `"15min,daily"`
    /// comma-separated pair) and updates the corresponding window. A missing
    /// or malformed header keeps the previous value for that field. The usage
    /// value is interpreted per the spec's [`UsageKind`]: `Used` stores it
    /// directly; `Remaining` derives usage as `limit - remaining`.
    ///
    /// Emits a WARN log on transition into (or out of) the state where any
    /// spec's header is missing from the response — a provider that should be
    /// sending its rate-limit headers but isn't leaves the limiter blind.
    pub fn update_from_headers(&mut self, headers: &HeaderMap) {
        let mut any_missing = false;
        for (window, spec) in self.windows.iter_mut().zip(self.specs.iter()) {
            let limit_pair = parse_pair(headers.get(&spec.limit_header));
            if limit_pair.is_none() {
                any_missing = true;
            }
            let usage_pair = parse_pair(headers.get(&spec.usage_header));
            if usage_pair.is_none() {
                any_missing = true;
            }
            if let Some(pair) = limit_pair {
                window.limit = Some(spec.cadence.pair_value(pair));
            }
            if let Some(pair) = usage_pair {
                let raw = spec.cadence.pair_value(pair);
                match spec.usage_kind {
                    UsageKind::Used => window.usage = Some(raw),
                    // `remaining` needs the limit to derive usage; without it
                    // the field keeps its previous value.
                    UsageKind::Remaining => {
                        if let Some(limit) = window.limit {
                            window.usage = Some(limit.saturating_sub(raw));
                        }
                    }
                }
            }
        }
        if any_missing && !self.was_missing {
            log::warn!(
                "{}: rate-limit headers missing from response; limiter state is stale",
                self.label
            );
        } else if !any_missing && self.was_missing {
            log::debug!("{}: rate-limit headers present again", self.label);
        }
        self.was_missing = any_missing;
        log::debug!("Rate limit state updated: {:?}", self.windows);
    }

    /// The latest boundary of any exhausted window, or `None` if budget
    /// remains (or no data yet). Pure: no logging, no mutation.
    pub fn gate(&self) -> Option<i64> {
        self.windows
            .iter()
            .zip(self.specs.iter())
            .filter(|(w, _)| w.is_exhausted())
            .map(|(_, spec)| spec.cadence.boundary())
            .max()
    }

    /// The latest boundary of any exhausted window, or `None` if budget
    /// remains (or no data yet).
    ///
    /// Emits a WARN log on transition into or out of the rate-limited state.
    pub fn limited_until(&mut self) -> Option<i64> {
        let until = self.gate();

        let is_limited = until.is_some();
        if is_limited && !self.was_limited {
            let states = self
                .windows
                .iter()
                .map(|w| w.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            log::warn!(
                "{} rate limit reached ({}); pausing until {}",
                self.label,
                states,
                until.unwrap_or(0),
            );
        } else if !is_limited && self.was_limited {
            let states = self
                .windows
                .iter()
                .map(|w| w.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            log::warn!("{} rate limit cleared ({})", self.label, states);
        }
        self.was_limited = is_limited;
        until
    }
}

/// The shared rate-limit shim: the scaffolding each provider module wraps
/// around the [`RateLimiter`] kernel — a process-wide limiter behind a
/// [`Mutex`], parameterized by the provider's label and [`WindowSpec`]s.
///
/// A provider keeps a `static` of this (constructed with its own label and
/// specs), its own per-thread test-override `thread_local` (a thread-local
/// can't live in a struct instance), and thin `limited_until` /
/// `update_from_headers` passthroughs to the shim's methods.
pub struct RateLimitShim(Mutex<RateLimiter>);

impl RateLimitShim {
    /// Create a new shim: a fresh [`RateLimiter`] with the given label and
    /// specs, behind the mutex.
    pub fn new(label: &'static str, specs: Vec<WindowSpec>) -> Self {
        Self(Mutex::new(RateLimiter::new(label, specs)))
    }

    /// The rate-limit gate: the unix time at which the exhausted window
    /// resets, or `None` if budget remains (or no data yet).
    ///
    /// Emits a WARN log on transition into or out of the rate-limited state.
    pub fn limited_until(&self) -> Option<i64> {
        self.0.lock().unwrap().limited_until()
    }

    /// Parses the provider's rate-limit response headers into the shared
    /// state. Each header is a `"15min,daily"` comma-separated pair of
    /// integers. Missing or malformed headers are ignored (the state keeps
    /// its previous values for those fields).
    pub fn update_from_headers(&self, headers: &HeaderMap) {
        self.0.lock().unwrap().update_from_headers(headers);
    }

    /// A fresh limiter in the shape of the process-wide one (the given label
    /// and specs), populated from the given `"header", "value"` pairs — the
    /// test helper a provider's per-thread override is built from.
    #[cfg(test)]
    pub fn limiter_from_headers(
        label: &'static str,
        specs: &[WindowSpec],
        pairs: &[(&str, &str)],
    ) -> RateLimiter {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            let name = HeaderName::from_bytes(name.as_bytes()).unwrap();
            headers.insert(name, HeaderValue::from_str(value).unwrap());
        }
        let mut limiter = RateLimiter::new(label, specs.to_vec());
        limiter.update_from_headers(&headers);
        limiter
    }
}

/// Parses a `"15min,daily"` comma-separated header value into a pair.
fn parse_pair(header: Option<&HeaderValue>) -> Option<(u32, u32)> {
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
mod tests {
    use super::*;

    /// A generic 4-window header mapping: 2 header-pairs, each with a
    /// 15-minute and a daily window.
    fn test_specs() -> Vec<WindowSpec> {
        vec![
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Usage").unwrap(),
                cadence: Cadence::FifteenMin,
                usage_kind: UsageKind::Used,
            },
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Usage").unwrap(),
                cadence: Cadence::Daily,
                usage_kind: UsageKind::Used,
            },
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Read-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Read-Usage").unwrap(),
                cadence: Cadence::FifteenMin,
                usage_kind: UsageKind::Used,
            },
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Read-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Read-Usage").unwrap(),
                cadence: Cadence::Daily,
                usage_kind: UsageKind::Used,
            },
        ]
    }

    /// A single-window spec whose usage header reports *remaining* requests.
    fn remaining_spec() -> Vec<WindowSpec> {
        vec![WindowSpec {
            limit_header: HeaderName::from_bytes(b"X-Limit").unwrap(),
            usage_header: HeaderName::from_bytes(b"X-Remaining").unwrap(),
            cadence: Cadence::FifteenMin,
            usage_kind: UsageKind::Remaining,
        }]
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut m = HeaderMap::new();
        for (k, v) in pairs {
            let name = HeaderName::from_bytes(k.as_bytes()).unwrap();
            m.insert(name, HeaderValue::from_str(v).unwrap());
        }
        m
    }

    fn window(limit: u32, usage: u32) -> Window {
        Window {
            limit: Some(limit),
            usage: Some(usage),
        }
    }

    // ─── Window ──────────────────────────────────────────────────────────

    #[test]
    fn window_exhausted_when_usage_equals_limit() {
        assert!(window(100, 100).is_exhausted());
    }

    #[test]
    fn window_not_exhausted_when_usage_below_limit() {
        assert!(!window(100, 99).is_exhausted());
    }

    #[test]
    fn window_not_exhausted_when_no_data() {
        assert!(!Window::default().is_exhausted());
    }

    #[test]
    fn window_display_formats_as_slash_pair() {
        assert_eq!(window(100, 80).to_string(), "80/100");
        assert_eq!(Window::default().to_string(), "0/0");
    }

    // ─── parse_pair ──────────────────────────────────────────────────────

    #[test]
    fn parse_pair_valid() {
        let h = HeaderValue::from_str("100,1000").unwrap();
        assert_eq!(parse_pair(Some(&h)), Some((100, 1000)));
    }

    #[test]
    fn parse_pair_none() {
        assert_eq!(parse_pair(None), None);
    }

    #[test]
    fn parse_pair_malformed() {
        let h = HeaderValue::from_str("not,a,number").unwrap();
        assert_eq!(parse_pair(Some(&h)), None);
    }

    // ─── Cadence ─────────────────────────────────────────────────────────

    #[test]
    fn cadence_boundary_15min_is_15min_mark() {
        let b = Cadence::FifteenMin.boundary();
        assert_eq!(b % 900, 0, "boundary is a 15-min mark");
        assert!(b > now_unix(), "boundary is in the future");
    }

    #[test]
    fn cadence_boundary_daily_is_midnight() {
        let b = Cadence::Daily.boundary();
        assert_eq!(b % 86400, 0, "boundary is a day mark");
        assert!(b > now_unix(), "boundary is in the future");
    }

    // ─── RateLimiter::update_from_headers ────────────────────────────────

    #[test]
    fn update_from_headers_parses_all_four() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[
            ("X-Limit", "200,2000"),
            ("X-Usage", "150,1500"),
            ("X-Read-Limit", "100,1000"),
            ("X-Read-Usage", "80,800"),
        ]);
        limiter.update_from_headers(&h);
        let w = limiter.windows();
        assert_eq!(w[0], window(200, 150));
        assert_eq!(w[1], window(2000, 1500));
        assert_eq!(w[2], window(100, 80));
        assert_eq!(w[3], window(1000, 800));
    }

    #[test]
    fn update_from_headers_missing_headers_keeps_previous() {
        let mut limiter = RateLimiter::new("test", test_specs());
        // First: populate overall windows.
        let h = headers(&[("X-Limit", "200,2000"), ("X-Usage", "150,1500")]);
        limiter.update_from_headers(&h);
        // Second: only read headers present; overall keeps its previous values.
        let h2 = headers(&[("X-Read-Limit", "100,1000"), ("X-Read-Usage", "80,800")]);
        limiter.update_from_headers(&h2);
        let w = limiter.windows();
        assert_eq!(w[0], window(200, 150));
        assert_eq!(w[1], window(2000, 1500));
        assert_eq!(w[2], window(100, 80));
        assert_eq!(w[3], window(1000, 800));
    }

    #[test]
    fn update_from_headers_malformed_value_ignored() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[("X-Limit", "not,a,number")]);
        limiter.update_from_headers(&h);
        let w = limiter.windows();
        assert_eq!(w[0], Window::default());
        assert_eq!(w[1], Window::default());
    }

    #[test]
    fn update_from_headers_empty_map_no_change() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[("X-Read-Limit", "100,1000"), ("X-Read-Usage", "42,420")]);
        limiter.update_from_headers(&h);
        let h2 = HeaderMap::new();
        limiter.update_from_headers(&h2);
        let w = limiter.windows();
        assert_eq!(w[2], window(100, 42));
    }

    // ─── UsageKind::Remaining ────────────────────────────────────────────

    #[test]
    fn remaining_zero_reads_as_exhausted() {
        let mut limiter = RateLimiter::new("test", remaining_spec());
        let h = headers(&[("X-Limit", "2500,5000"), ("X-Remaining", "0,4500")]);
        limiter.update_from_headers(&h);
        let w = limiter.windows();
        // usage = limit - remaining = 2500 - 0 = 2500 (15-min window)
        assert_eq!(w[0], window(2500, 2500));
        assert!(w[0].is_exhausted());
        assert!(limiter.gate().is_some());
    }

    #[test]
    fn remaining_positive_reads_as_not_exhausted() {
        let mut limiter = RateLimiter::new("test", remaining_spec());
        let h = headers(&[("X-Limit", "2500,5000"), ("X-Remaining", "100,4500")]);
        limiter.update_from_headers(&h);
        let w = limiter.windows();
        // usage = 2500 - 100 = 2400
        assert_eq!(w[0], window(2500, 2400));
        assert!(!w[0].is_exhausted());
        assert!(limiter.gate().is_none());
    }

    #[test]
    fn remaining_exceeding_limit_saturates_to_zero_usage() {
        let mut limiter = RateLimiter::new("test", remaining_spec());
        let h = headers(&[("X-Limit", "100,1000"), ("X-Remaining", "500,4500")]);
        limiter.update_from_headers(&h);
        let w = limiter.windows();
        // usage = 100 - 500 saturates to 0 (no underflow)
        assert_eq!(w[0], window(100, 0));
        assert!(!w[0].is_exhausted());
    }

    // ─── RateLimiter::limited_until ──────────────────────────────────────

    #[test]
    fn limited_until_no_data_returns_none() {
        let mut limiter = RateLimiter::new("test", test_specs());
        assert_eq!(limiter.limited_until(), None);
    }

    #[test]
    fn limited_until_budget_available_returns_none() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[
            ("X-Limit", "200,2000"),
            ("X-Usage", "80,800"),
            ("X-Read-Limit", "100,1000"),
            ("X-Read-Usage", "50,500"),
        ]);
        limiter.update_from_headers(&h);
        assert_eq!(limiter.limited_until(), None);
    }

    #[test]
    fn limited_until_read_15min_exhausted_returns_15min_boundary() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[
            ("X-Limit", "200,2000"),
            ("X-Usage", "150,800"),
            ("X-Read-Limit", "100,1000"),
            ("X-Read-Usage", "100,500"),
        ]);
        limiter.update_from_headers(&h);
        let until = limiter.limited_until().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(until > now, "boundary is in the future");
        assert_eq!(until % 900, 0, "boundary is a 15-min mark");
        assert!(until - now <= 900, "boundary is within 15 minutes");
    }

    #[test]
    fn limited_until_daily_exhausted_returns_midnight() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[
            ("X-Limit", "200,2000"),
            ("X-Usage", "80,1500"),
            ("X-Read-Limit", "100,1000"),
            ("X-Read-Usage", "50,1000"),
        ]);
        limiter.update_from_headers(&h);
        let until = limiter.limited_until().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(until > now, "midnight is in the future");
        assert_eq!(until % 86400, 0, "midnight is a day boundary");
        assert!(until - now <= 86400, "midnight is within 24 hours");
    }

    #[test]
    fn limited_until_both_exhausted_returns_later() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[
            ("X-Limit", "200,2000"),
            ("X-Usage", "200,2000"),
            ("X-Read-Limit", "100,1000"),
            ("X-Read-Usage", "100,1000"),
        ]);
        limiter.update_from_headers(&h);
        let until = limiter.limited_until().unwrap();
        let expected = next_15min_boundary().max(next_midnight_utc());
        assert_eq!(until, expected, "the latest boundary wins");
    }

    #[test]
    fn limited_until_overall_15min_exhausted() {
        let mut limiter = RateLimiter::new("test", test_specs());
        let h = headers(&[
            ("X-Limit", "200,2000"),
            ("X-Usage", "200,800"),
            ("X-Read-Limit", "100,1000"),
            ("X-Read-Usage", "50,500"),
        ]);
        limiter.update_from_headers(&h);
        assert!(limiter.limited_until().is_some());
    }
}
