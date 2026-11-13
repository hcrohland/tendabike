//! The icu HTTP test seam: a `thread_local` over the kit's canned-response
//! machinery (spec #838).
//!
//! Available in both build modes:
//! - `test-support` feature: consumer crates map canned responses for the URIs
//!   the loop asks for.
//! - `#[cfg(test)]`: the in-memory icu-logic tests queue per-thread canned
//!   responses and record the requested URIs they assert on.
//!
//! The common machinery (`CannedState`, `begin_test`, `queue`, `pop_test_canned`,
//! `requests`) lives in the kit's [`crate::test_support`]; this seam owns the
//! per-provider `thread_local` (a thread-local can't live in a struct instance)
//! and the thin free functions that reach it. There is no token cache to seed
//! (intervals.icu tokens are durable, read straight from `icu_users`), so the
//! seam is just the canned-response map plus the request recorder.

use std::cell::RefCell;

use super::super::*;
use crate::test_support::{self as kit, CannedState};

thread_local! {
    static CANNED: RefCell<CannedState> = RefCell::new(CannedState::default());
}

/// Begin a test's thread-local scope: marks the thread in test mode and clears
/// any canned responses/records a previous test left on this thread
/// (per-test isolation — each test runs on its own thread).
pub fn begin_test() {
    kit::begin_test(&CANNED);
}

/// Whether the current thread is in test mode.
pub fn in_test_mode() -> bool {
    kit::in_test_mode(&CANNED)
}

/// Queue a canned JSON response for an exact request URI (FIFO per URI).
pub fn queue(uri: &str, json: &str) {
    kit::queue(&CANNED, uri, json);
}

/// Queue an error for an exact request URI.
pub fn queue_error(uri: &str, err: Error) {
    kit::queue_error(&CANNED, uri, err);
}

/// Pop the canned response for `uri` (the free function's thread-local offline
/// seam).
///
/// Returns `None` when not in test mode (the caller falls through to the real
/// API). In test mode it always returns `Some`: the queued body (or error),
/// recording the request; an unqueued URI records the request and yields the
/// "unexpected request" error.
pub fn pop_test_canned(uri: &str) -> Option<TbResult<String>> {
    kit::pop_test_canned(&CANNED, uri, "icu")
}

/// The requested URIs, in order (assert on these to verify API calls).
pub fn requests() -> Vec<String> {
    kit::requests(&CANNED)
}
