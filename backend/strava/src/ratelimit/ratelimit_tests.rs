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
    assert_eq!(state.overall_limit_15min, Some(200));
    assert_eq!(state.overall_usage_15min, Some(150));
    assert_eq!(state.overall_limit_daily, Some(2000));
    assert_eq!(state.overall_usage_daily, Some(1500));
    assert_eq!(state.read_limit_15min, Some(100));
    assert_eq!(state.read_usage_15min, Some(80));
    assert_eq!(state.read_limit_daily, Some(1000));
    assert_eq!(state.read_usage_daily, Some(800));
}

#[test]
fn update_from_headers_missing_headers_keeps_previous() {
    let mut state = RateLimitState {
        overall_limit_15min: Some(200),
        overall_usage_15min: Some(150),
        ..Default::default()
    };
    // Only the read headers are present; overall keeps its previous values.
    let h = headers(&[
        ("X-ReadRateLimit-Limit", "100,1000"),
        ("X-ReadRateLimit-Usage", "80,800"),
    ]);
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.overall_limit_15min, Some(200));
    assert_eq!(state.overall_usage_15min, Some(150));
    assert_eq!(state.read_limit_15min, Some(100));
    assert_eq!(state.read_usage_15min, Some(80));
}

#[test]
fn update_from_headers_malformed_value_ignored() {
    let mut state = RateLimitState::default();
    let h = headers(&[("X-RateLimit-Limit", "not,a,number")]);
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.overall_limit_15min, None);
    assert_eq!(state.overall_limit_daily, None);
}

#[test]
fn update_from_headers_empty_map_no_change() {
    let mut state = RateLimitState {
        read_usage_15min: Some(42),
        ..Default::default()
    };
    let h = HeaderMap::new();
    update_from_headers_inner(&mut state, &h);
    assert_eq!(state.read_usage_15min, Some(42));
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
        read_usage_15min: Some(50),
        read_limit_15min: Some(100),
        read_usage_daily: Some(500),
        read_limit_daily: Some(1000),
        overall_usage_15min: Some(80),
        overall_limit_15min: Some(200),
        overall_usage_daily: Some(800),
        overall_limit_daily: Some(2000),
        ..Default::default()
    };
    assert_eq!(limited_until_inner(&state), None);
}

#[test]
fn limited_until_read_15min_exhausted_returns_15min_boundary() {
    let state = RateLimitState {
        read_usage_15min: Some(100),
        read_limit_15min: Some(100),
        read_usage_daily: Some(500),
        read_limit_daily: Some(1000),
        overall_usage_15min: Some(150),
        overall_limit_15min: Some(200),
        overall_usage_daily: Some(800),
        overall_limit_daily: Some(2000),
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
        read_usage_15min: Some(50),
        read_limit_15min: Some(100),
        read_usage_daily: Some(1000),
        read_limit_daily: Some(1000),
        overall_usage_15min: Some(80),
        overall_limit_15min: Some(200),
        overall_usage_daily: Some(1500),
        overall_limit_daily: Some(2000),
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
fn limited_until_both_exhausted_returns_earlier() {
    let state = RateLimitState {
        read_usage_15min: Some(100),
        read_limit_15min: Some(100),
        read_usage_daily: Some(1000),
        read_limit_daily: Some(1000),
        overall_usage_15min: Some(200),
        overall_limit_15min: Some(200),
        overall_usage_daily: Some(2000),
        overall_limit_daily: Some(2000),
        ..Default::default()
    };
    let until = limited_until_inner(&state).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!(
        until - now <= 900,
        "the 15-min boundary wins (it's earlier)"
    );
}

#[test]
fn limited_until_usage_equals_limit_is_exhausted() {
    let state = RateLimitState {
        read_usage_15min: Some(100),
        read_limit_15min: Some(100),
        ..Default::default()
    };
    assert!(
        limited_until_inner(&state).is_some(),
        "usage == limit means exhausted"
    );
}

#[test]
fn limited_until_usage_below_limit_is_not_exhausted() {
    let state = RateLimitState {
        read_usage_15min: Some(99),
        read_limit_15min: Some(100),
        ..Default::default()
    };
    assert_eq!(limited_until_inner(&state), None);
}

#[test]
fn limited_until_overall_15min_exhausted() {
    let state = RateLimitState {
        overall_usage_15min: Some(200),
        overall_limit_15min: Some(200),
        read_usage_15min: Some(50),
        read_limit_15min: Some(100),
        ..Default::default()
    };
    assert!(limited_until_inner(&state).is_some());
}
