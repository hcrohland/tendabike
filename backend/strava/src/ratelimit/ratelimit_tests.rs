use super::*;
use http::header::HeaderValue;

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut m = HeaderMap::new();
    for (k, v) in pairs {
        let name = http::HeaderName::from_bytes(k.as_bytes()).unwrap();
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

// ─── update_from_headers_inner ─────────────────────────────────────────

#[test]
fn update_from_headers_parses_all_four() {
    let mut state = RateLimitState::default();
    let h = headers(&[
        ("X-RateLimit-Limit", "200,2000"),
        ("X-RateLimit-Usage", "150,1500"),
        ("X-ReadRateLimit-Limit", "100,1000"),
        ("X-ReadRateLimit-Usage", "80,800"),
    ]);
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.overall_15min, window(200, 150));
    assert_eq!(state.overall_daily, window(2000, 1500));
    assert_eq!(state.read_15min, window(100, 80));
    assert_eq!(state.read_daily, window(1000, 800));
}

#[test]
fn update_from_headers_missing_headers_keeps_previous() {
    let mut state = RateLimitState {
        overall_15min: window(200, 150),
        overall_daily: window(2000, 1500),
        ..Default::default()
    };
    // Only the read headers are present; overall keeps its previous values.
    let h = headers(&[
        ("X-ReadRateLimit-Limit", "100,1000"),
        ("X-ReadRateLimit-Usage", "80,800"),
    ]);
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.overall_15min, window(200, 150));
    assert_eq!(state.overall_daily, window(2000, 1500));
    assert_eq!(state.read_15min, window(100, 80));
    assert_eq!(state.read_daily, window(1000, 800));
}

#[test]
fn update_from_headers_malformed_value_ignored() {
    let mut state = RateLimitState::default();
    let h = headers(&[("X-RateLimit-Limit", "not,a,number")]);
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.overall_15min, Window::default());
    assert_eq!(state.overall_daily, Window::default());
}

#[test]
fn update_from_headers_empty_map_no_change() {
    let mut state = RateLimitState {
        read_15min: window(100, 42),
        ..Default::default()
    };
    let h = HeaderMap::new();
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.read_15min, window(100, 42));
}

// ─── Window::is_exhausted ──────────────────────────────────────────────

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

// ─── limited_until_inner ───────────────────────────────────────────────

#[test]
fn limited_until_no_data_returns_none() {
    let state = RateLimitState::default();
    assert_eq!(limited_until_inner(&state), None);
}

#[test]
fn limited_until_budget_available_returns_none() {
    let state = RateLimitState {
        read_15min: window(100, 50),
        read_daily: window(1000, 500),
        overall_15min: window(200, 80),
        overall_daily: window(2000, 800),
        ..Default::default()
    };
    assert_eq!(limited_until_inner(&state), None);
}

#[test]
fn limited_until_read_15min_exhausted_returns_15min_boundary() {
    let state = RateLimitState {
        read_15min: window(100, 100),
        read_daily: window(1000, 500),
        overall_15min: window(200, 150),
        overall_daily: window(2000, 800),
        ..Default::default()
    };
    let until = limited_until_inner(&state).unwrap();
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
    let state = RateLimitState {
        read_15min: window(100, 50),
        read_daily: window(1000, 1000),
        overall_15min: window(200, 80),
        overall_daily: window(2000, 1500),
        ..Default::default()
    };
    let until = limited_until_inner(&state).unwrap();
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
    let state = RateLimitState {
        read_15min: window(100, 100),
        read_daily: window(1000, 1000),
        overall_15min: window(200, 200),
        overall_daily: window(2000, 2000),
        ..Default::default()
    };
    let until = limited_until_inner(&state).unwrap();
    // The latest boundary of the exhausted windows wins.
    let expected = next_15min_boundary().max(next_midnight_utc());
    assert_eq!(until, expected, "the latest boundary wins");
}

#[test]
fn limited_until_overall_15min_exhausted() {
    let state = RateLimitState {
        overall_15min: window(200, 200),
        read_15min: window(100, 50),
        ..Default::default()
    };
    assert!(limited_until_inner(&state).is_some());
}
