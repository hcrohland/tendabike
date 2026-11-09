//! Provider-agnostic rate-limit kernel: window model, header-pair parsing,
//! boundary math, and the "latest exhausted window" gate.
//!
//! A provider instantiates [`RateLimiter`] with its own [`WindowSpec`]s
//! (header names + cadence), getting an independent, process-wide limiter.
//! The state is in-memory: on restart it starts empty and learns from the
//! first response.

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

/// Ties one rate-limit window to its response headers and cadence.
#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub limit_header: HeaderName,
    pub usage_header: HeaderName,
    pub cadence: Cadence,
}

/// A per-provider rate limiter: one [`Window`] of state per [`WindowSpec`],
/// plus the `was_limited` transition tracker.
pub struct RateLimiter {
    label: &'static str,
    windows: Vec<Window>,
    specs: Vec<WindowSpec>,
    was_limited: bool,
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
    /// comma-separated pair) and updates the corresponding window. Missing or
    /// malformed headers keep the previous value for that field.
    pub fn update_from_headers(&mut self, headers: &HeaderMap) {
        for (window, spec) in self.windows.iter_mut().zip(self.specs.iter()) {
            if let Some(pair) = parse_pair(headers.get(&spec.limit_header)) {
                window.limit = Some(spec.cadence.pair_value(pair));
            }
            if let Some(pair) = parse_pair(headers.get(&spec.usage_header)) {
                window.usage = Some(spec.cadence.pair_value(pair));
            }
        }
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
            },
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Usage").unwrap(),
                cadence: Cadence::Daily,
            },
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Read-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Read-Usage").unwrap(),
                cadence: Cadence::FifteenMin,
            },
            WindowSpec {
                limit_header: HeaderName::from_bytes(b"X-Read-Limit").unwrap(),
                usage_header: HeaderName::from_bytes(b"X-Read-Usage").unwrap(),
                cadence: Cadence::Daily,
            },
        ]
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
