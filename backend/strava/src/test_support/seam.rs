//! The HTTP test seam: thread-local canned responses, the `TokenCache` seeder,
//! and the `CANNED_STRAVA` global.
//!
//! Available in both build modes:
//! - `test-support` feature: the `tb_exec`/`tb_sqlx` seam suites map canned
//!   responses for the URIs the real loop asks for.
//! - `#[cfg(test)]`: the in-memory Strava-logic tests queue per-thread canned
//!   responses and record the requested URIs / deauthorizations they assert on.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use oauth2::RefreshToken;
use tb_domain::{
    MockSession,
    test_support::{mock_session, mock_session_admin},
};

use crate::*;

// ─── Thread-local canned responses ──────────────────────────────────────────
//
// The thin free functions (`request_json`/`deauthorize`) take `&mut dyn
// Session` and cannot downcast to a scripted session, so the per-test
// canned responses live in a thread-local map the free functions consult.
// Each test runs on its own thread, so the map gives per-test isolation.
// `strava_session` (and the `tb_exec` gear tests) write to it; the free
// functions read from it. The process-wide `CANNED_STRAVA` global below is a
// separate seam for the seam suite's session, which runs in a spawned
// task on a thread this map never reaches.

/// The per-thread canned-response state (see the section docs).
#[derive(Default)]
struct CannedState {
    test_mode: bool,
    responses: HashMap<String, VecDeque<TbResult<String>>>,
    requests: Vec<String>,
    deauthorizes: Vec<StravaId>,
}

thread_local! {
    static CANNED: RefCell<CannedState> = RefCell::new(CannedState::default());
}

/// Begin a test's thread-local scope: marks the thread in test mode and clears
/// any canned responses/records a previous test left on this thread (per-test
/// isolation — each test runs on its own thread).
///
/// The process-wide `TokenCache` is *not* cleared here: the in-memory tests
/// run in parallel threads and all seed the same warm token (via
/// [`seed_token_cache`]), so a per-test clear would let one test's clear race
/// another test's `get_token` (a cache miss would force a real refresh).
pub fn begin_test() {
    CANNED.with(|c| {
        *c.borrow_mut() = CannedState::default();
        c.borrow_mut().test_mode = true;
    });
}

/// Seed the `TokenCache` with a warm token for `user_id` (test-only): prevents
/// the `get_token` free function from forcing a (network) refresh on the first
/// request (spec #602: the token source moved to `tb_strava`).
pub fn seed_token_cache(user_id: UserId, strava_id: StravaId, access_token: &str) {
    crate::session::TOKEN_CACHE
        .lock()
        .expect("the token cache is not poisoned")
        .insert(
            user_id,
            crate::session::TokenState::warm(strava_id, access_token),
        );
}

/// Whether the current thread is in test mode (`strava_session` was built,
/// or `begin_test` was called).
pub fn in_test_mode() -> bool {
    CANNED.with(|c| c.borrow().test_mode)
}

/// Queue a canned JSON response for an exact request URI (FIFO per URI).
pub fn queue(uri: &str, json: &str) {
    CANNED.with(|c| {
        c.borrow_mut()
            .responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Ok(json.to_string()));
    });
}

/// Queue an error for an exact request URI.
pub fn queue_error(uri: &str, err: Error) {
    CANNED.with(|c| {
        c.borrow_mut()
            .responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Err(err));
    });
}

/// Pop the canned response for `uri` (the free function's thread-local offline
/// seam).
///
/// Returns `None` when not in test mode (the caller falls through to the
/// process-wide `CANNED_STRAVA` global, then the real API). In test mode it
/// always returns `Some`: the queued body (or error), recording the request;
/// an unqueued URI records the request and yields the "unexpected request"
/// error.
pub fn pop_test_canned(uri: &str) -> Option<TbResult<String>> {
    CANNED.with(|c| {
        let mut c = c.borrow_mut();
        if !c.test_mode {
            return None;
        }
        c.requests.push(uri.to_string());
        Some(
            c.responses
                .get_mut(uri)
                .and_then(|q| q.pop_front())
                .unwrap_or(Err(Error::BadRequest(format!(
                    "unexpected strava request: {uri}"
                )))),
        )
    })
}

/// Record a deauthorization in the thread-local map (the free function's
/// offline seam).
pub fn record_deauthorize(strava: StravaId) {
    CANNED.with(|c| c.borrow_mut().deauthorizes.push(strava));
}

/// The requested URIs, in order (assert on these to verify API calls).
pub fn requests() -> Vec<String> {
    CANNED.with(|c| c.borrow().requests.clone())
}

/// The recorded `deauthorize()` calls.
pub fn deauthorizes() -> Vec<StravaId> {
    CANNED.with(|c| c.borrow().deauthorizes.clone())
}

/// A [`MockSession`] for `user` with a warm token seeded for `strava`
/// (test-only, no refresh): the thin free functions' `request_json`/
/// `deauthorize` replay the thread-local canned responses and record the
/// requested URIs / deauthorizations the tests assert on (the `queue`/
/// `requests`/`deauthorizes` free functions below).
pub fn strava_session(user: UserId, strava: StravaId) -> MockSession {
    begin_test();
    seed_token_cache(user, strava, "test-access-token");
    mock_session(user)
}

/// An admin [`MockSession`] for `user` with a warm token seeded for `strava`
/// (the admin-verification tests): like [`strava_session`], with `is_admin()`
/// returning `true`.
pub fn strava_admin_session(user: UserId, strava: StravaId) -> MockSession {
    begin_test();
    seed_token_cache(user, strava, "test-access-token");
    mock_session_admin(user)
}

/// Builds a [`StravaUser`] for tests.
pub fn strava_user(tbid: UserId, stravaid: i32, enabled: bool) -> StravaUser {
    StravaUser {
        id: stravaid.into(),
        tendabike_id: tbid,
        refresh_token: enabled.then(|| RefreshToken::new("test-token".to_string())),
    }
}

/// Minimal Strava activity JSON with the given id, type, and gear id.
pub fn activity_json(id: i64, typ: &str, gear: Option<&str>) -> String {
    let gear = match gear {
        Some(g) => format!("\"{g}\""),
        None => "null".to_string(),
    };
    format!(
        r#"{{"id":{id},"type":"{typ}","name":"Test Ride","start_date":"2026-01-02T10:00:00Z","utc_offset":0,"elapsed_time":3600,"moving_time":3300,"distance":25000.0,"total_elevation_gain":300.0,"kilojoules":4000.0,"gear_id":{gear},"device_name":"Test Device","external_id":null}}"#
    )
}

/// Minimal Strava gear JSON with the given id; `frame_type` None means shoes.
pub fn gear_json(id: &str, frame_type: Option<i32>) -> String {
    let ft = frame_type
        .map(|t| t.to_string())
        .unwrap_or_else(|| "null".into());
    format!(
        r#"{{"id":"{id}","name":"Test Bike","brand_name":"Test Brand","model_name":"Test Model","frame_type":{ft}}}"#
    )
}

// ─── Canned Strava responses (feature `test-support`) ──────────────────────
//
// The seam suite runs the real loop, whose session is a plain `Session`, so
// the suite maps canned responses for the URIs the loop asks for — an empty
// activity list makes the sync's activities fetch succeed offline (no new
// activities: the loop's processing consumes the event and commits).
// `request_json` consults [`global_canned_response`] before reaching the
// Strava API.

/// The canned Strava responses, keyed by URI (see the section docs).
pub static CANNED_STRAVA: std::sync::LazyLock<std::sync::Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Maps a canned response for `uri` (test-only, see [`CANNED_STRAVA`]).
pub fn canned_strava(uri: &str, json: &str) {
    CANNED_STRAVA
        .lock()
        .expect("the canned Strava responses are not poisoned")
        .insert(uri.to_string(), json.to_string());
}

/// The process-wide canned response for `uri`, if one is mapped (test-only,
/// the seam-suite's global seam — distinct from the thread-local
/// [`pop_test_canned`]).
pub fn global_canned_response(uri: &str) -> Option<String> {
    CANNED_STRAVA
        .lock()
        .expect("the canned Strava responses are not poisoned")
        .get(uri)
        .cloned()
}
