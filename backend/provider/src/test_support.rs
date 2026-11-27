//! Test support for `tb_provider` (spec #781, #838).
//!
//! - The [`MockEntityFacade`] double: the `mockall` `#[automock]` witness for
//!   [`EntityFacade`], available under the `test-support` feature (and
//!   `cfg(test)`). Provider tests mock this seam instead of the concrete
//!   domain ops — the domain logic runs for real behind the concrete
//!   [`EntityFacadeImpl`].
//! - The **canned-response wiring** the provider HTTP test seams share
//!   (spec #838): [`CannedState`] (the per-thread state), the single shared
//!   thread-local map, and the free functions over it ([`begin_test`],
//!   [`in_test_mode`], [`queue`], [`queue_error`], [`pop_test_canned`],
//!   [`requests`]). The kit owns the thread-local and the accessors; a
//!   driver's seam keeps only its fixture constructors and its
//!   provider-specific canned data.
//! - The **dummy oauth client** the driver and token test seams share
//!   (spec #1062): the [`DummyClient`] type alias (generic over the
//!   token-response type), [`dummy_client`] (the owned build), and
//!   [`basic_dummy_client`] (the shared `&'static` for the
//!   [`BasicTokenResponse`] shape).
//!
//! [`EntityFacade`]: crate::EntityFacade
//! [`EntityFacadeImpl`]: crate::EntityFacadeImpl

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;

use oauth2::{
    AuthUrl, ClientId, EndpointNotSet, EndpointSet, TokenResponse, TokenUrl,
    basic::BasicTokenResponse,
};

use crate::client::ProviderClient;
use crate::facade::{Error, TbResult};

pub use crate::facade::MockEntityFacade;
pub use crate::store::MockStore;

/// The test process's shared webhook secret (the intake tests verify
/// deliveries against it).
pub const ICU_WEBHOOK_SECRET: &str = "icu-test-webhook-secret";

/// Initialize the provider config exactly once for the test process: both
/// providers enabled (their ops' graceful-freeze guard reads the enabled
/// set) and the known [`ICU_WEBHOOK_SECRET`]. `crate::init` panics on a
/// second call, so every test suite that needs the config shares this one
/// initializer (a per-suite `Once` would race with the others').
pub fn init_test_config_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        crate::init(crate::ProviderConfig {
            strava_client_id: Some("test-client-id".to_string()),
            strava_client_secret: Some("test-client-secret".to_string()),
            strava_redirect_url: "http://localhost:8000/strava/token".to_string(),
            icu_client_id: Some("test-icu-client-id".to_string()),
            icu_client_secret: Some("test-icu-client-secret".to_string()),
            icu_redirect_url: "http://localhost:8000/icu/token".to_string(),
            icu_webhook_secret: ICU_WEBHOOK_SECRET.to_string(),
        });
    });
}

/// The suite's shared webhook secret (initializes the provider config via
/// the shared process-wide initializer).
pub fn icu_webhook_secret() -> String {
    init_test_config_once();
    ICU_WEBHOOK_SECRET.to_string()
}

/// The per-thread canned-response state the provider HTTP test seams share:
/// the single thread-local map the kit's accessors reach.
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

// The single shared thread-local canned-response map (per-test isolation:
// each test runs on its own thread, so the map is fresh per test). A
// thread-local can't live in a struct instance, so the kit owns it here and
// the accessors below reach it directly.
thread_local! {
    static CANNED: RefCell<CannedState> = RefCell::new(CannedState::default());
}

/// Begin a test's thread-local scope: resets the state to a fresh
/// [`CannedState`] and marks the thread in test mode (per-test isolation —
/// each test runs on its own thread).
pub fn begin_test() {
    CANNED.with(|c| {
        *c.borrow_mut() = CannedState::default();
        c.borrow_mut().test_mode = true;
    });
}

/// Whether the thread is in test mode.
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

/// Pop the canned response for `uri` (the free function's thread-local
/// offline seam).
///
/// Returns `None` when not in test mode (the caller falls through to its
/// other offline seams, then the real API). In test mode it always returns
/// `Some`: the queued body (or error), recording the request; an unqueued URI
/// records the request and yields the "unexpected `{provider}` request"
/// error.
pub fn pop_test_canned(uri: &str, provider: &str) -> Option<TbResult<String>> {
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
                    "unexpected {provider} request: {uri}"
                )))),
        )
    })
}

/// The requested URIs, in order (assert on these to verify API calls).
pub fn requests() -> Vec<String> {
    CANNED.with(|c| c.borrow().requests.clone())
}

/// The dummy oauth client type the driver and token test seams share
/// (spec #1062): the common provider client over the token-response type,
/// with the endpoint markers left generic (so `::new` — defined only on the
/// all-unset client — is resolved through the un-instantiated markers, and
/// the setter chain flips them).
pub type DummyClient<
    TokenResponse,
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointSet,
> = ProviderClient<
    TokenResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

/// Build a dummy oauth client (locally-pointing auth/token URLs): a client
/// that satisfies the [`OauthClient`](crate::client::OauthClient) bound for
/// tests that never reach the network — the token hooks and exchange
/// branches return before a request is sent.
pub fn dummy_client<T: TokenResponse>(client_id: &str) -> DummyClient<T> {
    DummyClient::<
        T,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
    >::new(ClientId::new(client_id.to_string()))
    .set_auth_uri(AuthUrl::new("http://localhost/oauth/authorize".to_string()).unwrap())
    .set_token_uri(TokenUrl::new("http://localhost/oauth/token".to_string()).unwrap())
}

/// The shared dummy oauth client for the [`BasicTokenResponse`] shape: the
/// `&'static` the driver trait's `client()` returns, shared by every
/// provider-agnostic test module whose `Client` is over the basic token
/// response (the `fn client() -> &'static` forces a per-type static, so the
/// shared one for this shape lives here).
pub fn basic_dummy_client() -> &'static DummyClient<BasicTokenResponse> {
    static CLIENT: OnceLock<DummyClient<BasicTokenResponse>> = OnceLock::new();
    CLIENT.get_or_init(|| dummy_client("test"))
}
