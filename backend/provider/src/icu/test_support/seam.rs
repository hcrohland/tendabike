//! The icu HTTP test seam: thread-local canned responses.
//!
//! Available in both build modes:
//! - `test-support` feature: consumer crates map canned responses for the URIs
//!   the loop asks for.
//! - `#[cfg(test)]`: the in-memory icu-logic tests queue per-thread canned
//!   responses and record the requested URIs they assert on.
//!
//! Unlike the Strava driver there is no token cache to seed (intervals.icu
//! tokens are durable, read straight from `icu_users`), so the seam is just
//! the canned-response map plus the request recorder.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use super::super::*;

/// The per-thread canned-response state.
#[derive(Default)]
struct CannedState {
    test_mode: bool,
    responses: HashMap<String, VecDeque<TbResult<String>>>,
    requests: Vec<String>,
}

thread_local! {
    static CANNED: RefCell<CannedState> = RefCell::new(CannedState::default());
}

/// Begin a test's thread-local scope: marks the thread in test mode and clears
/// any canned responses/records a previous test left on this thread
/// (per-test isolation — each test runs on its own thread).
pub fn begin_test() {
    CANNED.with(|c| {
        *c.borrow_mut() = CannedState::default();
        c.borrow_mut().test_mode = true;
    });
}

/// Whether the current thread is in test mode.
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
/// Returns `None` when not in test mode (the caller falls through to the real
/// API). In test mode it always returns `Some`: the queued body (or error),
/// recording the request; an unqueued URI records the request and yields the
/// "unexpected request" error.
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
                    "unexpected icu request: {uri}"
                )))),
        )
    })
}

/// The requested URIs, in order (assert on these to verify API calls).
pub fn requests() -> Vec<String> {
    CANNED.with(|c| c.borrow().requests.clone())
}
