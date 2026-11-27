//! The provider-agnostic token-refresh machinery (spec #1056): the
//! per-provider [`TokenState`] (the owner id plus the token fields the
//! refresh drives), the process-wide [`TokenCache`], and the generic
//! [`get_token`] / [`get_owner_id`] the driver's thin forwards drive.
//!
//! The kit owns the cache, the cache lookup (a hit clones; a miss builds
//! from the DB row and inserts), the refresh-if-expired flow, and the
//! rotated-token persist (through the store's generic
//! [`user_update_token`](Store::user_update_token) op). The driver supplies
//! only its refresh exchange and its refresh-token accessor, as
//! [`TokenRefresh`] associated functions — the exchange is required (a
//! durable-token provider overrides it with a no-op) and the accessor
//! defaults to `None`.
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
use oauth2::{AccessToken, RefreshToken};

use crate::ProviderSpec;
use crate::facade::{Error, TbResult, UserId};
use crate::store::Store;

/// The driver's token-refresh hooks (spec #1056): the exchange and the
/// refresh-token accessor the kit's generic refresh flow reaches through the
/// marker's `Driver` bound. A dedicated, narrow trait — the kit's token
/// machinery uses only these two functions, so a `ProviderSpec`'s `Driver`
/// bound names this trait rather than the full driver, and a durable-token
/// provider (or a test stub) implements nothing at all (both default).
#[async_trait::async_trait]
pub trait TokenRefresh<P: ProviderSpec>: Send + Sync {
    /// Refresh the user's access token (spec #1056): take the current
    /// refresh token, perform the exchange using the driver's client, and
    /// return the kit's [`RefreshedToken`] (the new access token, the
    /// optional expiry duration, and the optional rotated refresh token —
    /// the signal for whether a persist is needed).
    ///
    /// The core of a `TokenRefresh`: a provider whose tokens are durable
    /// (intervals.icu) overrides it with a no-op (its tokens never refresh;
    /// the kit's refresh flow never reaches it).
    async fn refresh(refresh_token: RefreshToken) -> TbResult<RefreshedToken>;

    /// Read the refresh token out of the user row (spec #1056) — the durable
    /// fallback the token cache's miss builds from.
    ///
    /// Default: `None` (a provider whose tokens are durable has no refresh
    /// token).
    fn refresh_token(_user: &P::User) -> Option<RefreshToken> {
        None
    }
}

/// The result of a token refresh (spec #1056): the new access token, the
/// optional expiry duration, and the optional rotated refresh token — the
/// signal for whether a persist is needed.
///
/// `pub` (the fields `pub(crate)`): it is the return type of the
/// [`TokenRefresh::refresh`] associated function on the public driver
/// trait.
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
    /// driver's [`TokenRefresh::refresh_token`] accessor (the durable
    /// fallback).
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
    /// driver's [`TokenRefresh::refresh`] exchange). Returns the *new*
    /// refresh token only when a refresh issued a new rotating token (the
    /// signal for whether a persist is needed).
    async fn refresh_if_expired(&mut self) -> TbResult<Option<RefreshToken>> {
        if !is_expired(&self.expires_at) {
            return Ok(None);
        }

        let current_refresh = self.refresh_token.clone().ok_or_else(|| {
            Error::NotAuth("No Refresh Token provided and Access Token expired".to_string())
        })?;
        debug!("refreshing provider token");
        let token = P::Driver::refresh(current_refresh).await?;

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
}
