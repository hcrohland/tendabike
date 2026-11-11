//! Test support for `tb_provider` (spec #781, #838).
//!
//! - The [`MockEntityFacade`] double: the `mockall` `#[automock]` witness for
//!   [`EntityFacade`], available under the `test-support` feature (and
//!   `cfg(test)`). Provider tests mock this seam instead of the concrete
//!   domain ops — the domain logic runs for real behind the concrete
//!   [`EntityFacadeImpl`].
//! - The **common canned-response machinery** the provider HTTP test seams
//!   share (spec #838): [`CannedState`] (the per-thread state) and the free
//!   functions over it ([`begin_test`], [`in_test_mode`], [`queue`],
//!   [`queue_error`], [`pop_test_canned`], [`requests`]). Each provider seam
//!   owns its own `thread_local` (a thread-local can't live in a struct
//!   instance) and thin free functions that reach it through these.
//!
//! [`EntityFacade`]: crate::EntityFacade
//! [`EntityFacadeImpl`]: crate::EntityFacadeImpl

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::thread::LocalKey;

use tb_domain::{Error, TbResult};

pub use crate::facade::MockEntityFacade;

/// The per-thread canned-response state a provider's HTTP test seam owns.
///
/// The common fields every seam needs: whether the thread is in test mode,
/// the queued responses (FIFO per URI), and the recorded request URIs.
/// Provider-specific extras (e.g. Strava's deauthorization recorder) live in
/// the provider's seam, not here — this struct has no provider-specific
/// fields, so it is shared verbatim by every seam.
#[derive(Default)]
pub struct CannedState {
    test_mode: bool,
    responses: HashMap<String, VecDeque<TbResult<String>>>,
    requests: Vec<String>,
}

/// Begin a test's thread-local scope on the seam's `CANNED` key: resets the
/// state to a fresh [`CannedState`] and marks the thread in test mode
/// (per-test isolation — each test runs on its own thread).
///
/// `key` is a `&'static` because the seam's `thread_local!` static is
/// process-wide; `LocalKey::with` requires a `'static` receiver.
pub fn begin_test(key: &'static LocalKey<RefCell<CannedState>>) {
    key.with(|c| {
        *c.borrow_mut() = CannedState::default();
        c.borrow_mut().test_mode = true;
    });
}

/// Whether the thread is in test mode on the seam's `CANNED` key.
pub fn in_test_mode(key: &'static LocalKey<RefCell<CannedState>>) -> bool {
    key.with(|c| c.borrow().test_mode)
}

/// Queue a canned JSON response for an exact request URI (FIFO per URI).
pub fn queue(key: &'static LocalKey<RefCell<CannedState>>, uri: &str, json: &str) {
    key.with(|c| {
        c.borrow_mut()
            .responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Ok(json.to_string()));
    });
}

/// Queue an error for an exact request URI.
pub fn queue_error(key: &'static LocalKey<RefCell<CannedState>>, uri: &str, err: Error) {
    key.with(|c| {
        c.borrow_mut()
            .responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Err(err));
    });
}

/// Pop the canned response for `uri` (the free function's thread-local
/// offline seam).
///
/// Returns `None` when not in test mode (the caller falls through to its
/// other offline seams, then the real API). In test mode it always returns
/// `Some`: the queued body (or error), recording the request; an unqueued URI
/// records the request and yields the "unexpected `{provider}` request"
/// error.
pub fn pop_test_canned(
    key: &'static LocalKey<RefCell<CannedState>>,
    uri: &str,
    provider: &str,
) -> Option<TbResult<String>> {
    key.with(|c| {
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
                    "unexpected {provider} request: {uri}"
                )))),
        )
    })
}

/// The requested URIs, in order (assert on these to verify API calls).
pub fn requests(key: &'static LocalKey<RefCell<CannedState>>) -> Vec<String> {
    key.with(|c| c.borrow().requests.clone())
}
