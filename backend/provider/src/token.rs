//! The provider-agnostic token-refresh machinery (spec #1056, consolidated
//! in spec #1062): the per-provider [`TokenState`] (the owner id plus the
//! token fields the refresh drives), the process-wide [`TokenCache`], the
//! kit's [`refresh_exchange`] (the standard oauth2 refresh-token call,
//! written once), and the generic [`get_token`] / [`get_owner_id`] the
//! driver's thin forwards drive.
//!
//! The kit owns the cache, the cache lookup (a hit clones; a miss builds
//! from the DB row and inserts), the refresh-if-expired flow, the refresh
//! exchange, and the rotated-token persist (through the store's generic
//! [`user_update_token`](Store::user_update_token) op). The driver supplies
//! its refresh-token accessor (default: absent) and its shared client (the
//! exchange's input) as [`Provider`](crate::driver::Provider) associated
//! functions; the driver's `refresh` default is the exchange, so no
//! provider re-implements it — a durable-token provider's token path reads
//! the durable token directly from the user row and never reaches this
//! flow.
//!
//! The generic types are state-free: a provider keeps a named `static`
//! [`TokenCache`] and the kit reaches it by borrowing a reference (the same
//! shape as the rate-limit handle). The DB keeps the refresh token as the
//! durable fallback; on a cache miss the [`TokenState`] is built from the
//! DB's refresh token (an empty access token with a past expiry), forcing
//! the first request to refresh.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use log::debug;
use oauth2::{AccessToken, RefreshToken, TokenResponse};

use crate::ProviderSpec;
use crate::client::{HTTP_CLIENT, OauthClient};
use crate::driver::Provider;
use crate::facade::{Error, TbResult, UserId};
use crate::store::Store;

/// The result of a token refresh (spec #1056): the new access token, the
/// optional expiry duration, and the optional rotated refresh token — the
/// signal for whether a persist is needed.
///
/// `pub` (the fields `pub(crate)`): it is the return type of the driver
/// trait's [`refresh`](crate::driver::Provider::refresh) method — publicly
/// reachable through the driver module (re-exported there).
#[derive(Debug, Clone)]
pub struct RefreshedToken {
    /// The new access token.
    pub(crate) access_token: AccessToken,
    /// The access token's lifetime from now, if the provider reports one.
    pub(crate) expires_in: Option<Duration>,
    /// The rotated refresh token, if the exchange issued a new one (the
    /// signal for whether a persist is needed).
    pub(crate) refresh_token: Option<RefreshToken>,
}

/// The refresh exchange (spec #1056, consolidated into the kit's free
/// function in spec #1062): the standard oauth2 refresh-token call, written
/// once, generic over the provider client's token response type (the
/// [`OauthClient`] bound names the shape).
///
/// Takes the provider's shared oauth2 client and the optional refresh token:
/// an absent token is the `NotAuth` — the single home of the no-refresh-token
/// rule — an exchange failure maps to `NotAuth`, and the success returns the
/// [`RefreshedToken`] (the new access token, the optional expiry, the
/// optional rotated refresh token — the persist signal).
pub(crate) async fn refresh_exchange<C>(
    client: &C,
    refresh_token: Option<RefreshToken>,
) -> TbResult<RefreshedToken>
where
    C: OauthClient,
{
    let Some(refresh_token) = refresh_token else {
        return Err(Error::NotAuth(
            "No Refresh Token provided and Access Token expired".to_string(),
        ));
    };

    let token = client
        .exchange_refresh_token(&refresh_token)
        .request_async(&*HTTP_CLIENT)
        .await
        .map_err(|err| Error::NotAuth(err.to_string()))?;

    Ok(RefreshedToken {
        access_token: token.access_token().clone(),
        expires_in: token.expires_in(),
        refresh_token: token.refresh_token().cloned(),
    })
}

/// A one-line expiry check: a known expiry in the past is expired; an unknown
/// expiry (`None`) is treated as not expired.
fn is_expired(expires_at: &Option<SystemTime>) -> bool {
    expires_at.is_some_and(|e| SystemTime::now() > e)
}

/// The token state for a single provider user (spec #1056): the owner id
/// plus the token fields the refresh machinery drives.
///
/// Private to the kit: no token type or value crosses the module boundary
/// (the driver's thin forwards return the owner id and the access token).
/// Built from the DB's durable refresh token on a cache miss (an empty
/// access token with a past expiry, so the first request forces a refresh).
#[derive(Debug)]
pub(crate) struct TokenState<P: ProviderSpec> {
    owner: P::Owner,
    access_token: AccessToken,
    expires_at: Option<SystemTime>,
    refresh_token: Option<RefreshToken>,
}

impl<P: ProviderSpec> Clone for TokenState<P> {
    fn clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            access_token: self.access_token.clone(),
            expires_at: self.expires_at,
            refresh_token: self.refresh_token.clone(),
        }
    }
}

impl<P: ProviderSpec> TokenState<P> {
    /// Build the token state from a stored user row: an empty access token
    /// with a past expiry, so the first request forces a token refresh
    /// (spec #1056). The refresh token is read out of the row by the
    /// driver's
    /// [`refresh_token`](crate::driver::Provider::refresh_token) accessor
    /// (the durable fallback).
    fn from_user(user: &P::User) -> Self {
        Self {
            owner: P::user_owner(user).clone(),
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token: P::Driver::refresh_token(user),
        }
    }

    /// A warm token state (test-only): a valid access token with a future
    /// expiry, so no refresh is needed.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn warm(owner: P::Owner, access_token: &str) -> Self {
        Self {
            owner,
            access_token: AccessToken::new(access_token.to_string()),
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
            refresh_token: Some(RefreshToken::new("test-token".to_string())),
        }
    }

    /// The user's owner id.
    pub(crate) fn owner(&self) -> &P::Owner {
        &self.owner
    }

    /// The access token.
    pub(crate) fn access_token(&self) -> &AccessToken {
        &self.access_token
    }

    /// Ensure a valid access token, refreshing in place if expired (the
    /// driver's
    /// [`refresh`](crate::driver::Provider::refresh) — the kit's exchange).
    /// Returns the *new* refresh token only when a refresh issued a new
    /// rotating token (the signal for whether a persist is needed).
    ///
    /// The no-refresh-token rule is not here: the option is passed through,
    /// and the exchange is its single home (an absent token is the
    /// exchange's `NotAuth`).
    async fn refresh_if_expired(&mut self) -> TbResult<Option<RefreshToken>> {
        if !is_expired(&self.expires_at) {
            return Ok(None);
        }

        debug!("refreshing provider token");
        let token = P::Driver::refresh(self.refresh_token.clone()).await?;

        self.access_token = token.access_token.clone();
        self.expires_at = token.expires_in.map(|d| SystemTime::now() + d);
        let rotated = token.refresh_token;
        self.refresh_token = rotated.clone();
        Ok(rotated)
    }
}

/// The process-wide in-memory token cache (spec #1056), keyed by
/// [`UserId`]. The DB keeps the refresh token as the durable fallback; on a
/// cache miss the [`TokenState`] is built from the DB's refresh token,
/// forcing a refresh.
///
/// State-free apart from the map: a provider keeps a named `static` of this
/// and the kit's generic ops reach it by borrowing a reference.
pub(crate) struct TokenCache<P: ProviderSpec>(Mutex<HashMap<UserId, TokenState<P>>>);

impl<P: ProviderSpec> TokenCache<P> {
    /// Create a new, empty cache.
    pub(crate) fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    /// The cache lookup (spec #1056): a cache hit clones the warm
    /// [`TokenState`]; a cache miss builds it from the DB's refresh token
    /// (and inserts it), forcing a refresh on the first request.
    ///
    /// The `MutexGuard` is not `Send` and must not be held across the
    /// `await`: the lookup lock is scoped to its own block (cloned on a
    /// hit), and the miss path reads the user row with no lock held, then
    /// locks again to insert the new state.
    async fn lookup<S: Store<P>>(&self, user_id: UserId, store: &mut S) -> TbResult<TokenState<P>> {
        let cached = {
            let cache = self.0.lock().expect("the token cache is not poisoned");
            cache.get(&user_id).cloned()
        };
        match cached {
            Some(state) => Ok(state),
            None => {
                let user = store.user_get_by_tbid(user_id).await?;
                let state = TokenState::from_user(&user);
                let mut cache = self.0.lock().expect("the token cache is not poisoned");
                cache.insert(user_id, state.clone());
                Ok(state)
            }
        }
    }

    /// Store a token state (after a refresh, or from the seeding seam).
    pub(crate) fn put(&self, user_id: UserId, state: TokenState<P>) {
        let mut cache = self.0.lock().expect("the token cache is not poisoned");
        cache.insert(user_id, state);
    }
}

/// A valid access token for `user_id` (spec #1056): a cache hit clones the
/// warm [`TokenState`]; a cache miss builds it from the DB's refresh token
/// (and inserts it), forcing a refresh. A refresh always stores the durable
/// refresh token: when the exchange issues a new rotating token it is
/// persisted to the DB via the store's `update_token` op before the access
/// token is returned, so a refresh can never happen without the rotated
/// token being stored. Returns `(owner, access)`.
pub(crate) async fn get_token<P: ProviderSpec, S: Store<P>>(
    user_id: UserId,
    cache: &TokenCache<P>,
    store: &mut S,
) -> TbResult<(P::Owner, AccessToken)> {
    let mut state = cache.lookup(user_id, store).await?;
    let rotated = state.refresh_if_expired().await?;
    let owner = state.owner().clone();
    if let Some(refresh) = rotated {
        store
            .user_update_token(owner.clone(), Some(refresh.secret().to_string()))
            .await?;
    }
    let access = state.access_token().clone();
    cache.put(user_id, state);
    Ok((owner, access))
}

/// The user's owner id (spec #1056): the no-refresh sibling of
/// [`get_token`]. A cache hit returns the cached [`TokenState`]'s owner; a
/// cache miss builds it from the DB's user row (and inserts it). It is a
/// read-only owner resolution: it never refreshes.
pub(crate) async fn get_owner_id<P: ProviderSpec, S: Store<P>>(
    user_id: UserId,
    cache: &TokenCache<P>,
    store: &mut S,
) -> TbResult<P::Owner> {
    let state = cache.lookup(user_id, store).await?;
    Ok(state.owner().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ProviderClient;
    use oauth2::{
        AuthUrl, ClientId, EndpointNotSet, EndpointSet, TokenUrl, basic::BasicTokenResponse,
    };
    use std::sync::OnceLock;

    /// The dummy client's type: the common provider client over the basic
    /// token response, with the endpoint markers left generic (so `::new` —
    /// defined only on the all-unset client — is resolved through the
    /// un-instantiated markers, and the setter chain flips them).
    type DummyClient<
        HasAuthUrl = EndpointSet,
        HasDeviceAuthUrl = EndpointNotSet,
        HasIntrospectionUrl = EndpointNotSet,
        HasRevocationUrl = EndpointNotSet,
        HasTokenUrl = EndpointSet,
    > = ProviderClient<
        BasicTokenResponse,
        HasAuthUrl,
        HasDeviceAuthUrl,
        HasIntrospectionUrl,
        HasRevocationUrl,
        HasTokenUrl,
    >;

    /// A dummy oauth client (locally-pointing auth/token URLs): the
    /// absent-token exchange test needs a client that satisfies the
    /// [`OauthClient`] bound — the branch never reaches the network.
    fn dummy_client() -> &'static DummyClient {
        static CLIENT: OnceLock<DummyClient> = OnceLock::new();
        CLIENT.get_or_init(|| {
            DummyClient::new(ClientId::new("dummy".to_string()))
                .set_auth_uri(AuthUrl::new("http://localhost/oauth/authorize".to_string()).unwrap())
                .set_token_uri(TokenUrl::new("http://localhost/oauth/token".to_string()).unwrap())
        })
    }

    /// A known expiry in the past is expired (the cache-miss state's
    /// `UNIX_EPOCH` expiry always reads as expired).
    #[test]
    fn is_expired_past_expiry_is_expired() {
        assert!(is_expired(&Some(SystemTime::UNIX_EPOCH)));
    }

    /// A known expiry in the future is not expired (the warm state's
    /// `now + 1h` expiry never reads as expired).
    #[test]
    fn is_expired_future_expiry_is_not_expired() {
        let future = SystemTime::now() + Duration::from_secs(3600);
        assert!(!is_expired(&Some(future)));
    }

    /// An unknown expiry (`None`) is treated as not expired.
    #[test]
    fn is_expired_no_expiry_is_not_expired() {
        assert!(!is_expired(&None));
    }

    /// The absent-refresh-token branch: no refresh token is the `NotAuth`
    /// (the single home of the no-refresh-token rule) — the message is the
    /// existing one, preserved verbatim. Pure: the branch returns before the
    /// exchange touches the client, so no network and no store.
    #[tokio::test]
    async fn refresh_exchange_absent_token_is_not_auth() {
        let res = refresh_exchange(dummy_client(), None).await;
        assert!(
            matches!(
                res,
                Err(Error::NotAuth(ref msg))
                    if msg == "No Refresh Token provided and Access Token expired"
            ),
            "the absent token is the exchange's NotAuth, got {res:?}"
        );
    }
}
