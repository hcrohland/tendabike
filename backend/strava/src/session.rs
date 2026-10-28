/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! The `tb_strava`-owned Strava token machinery (spec #602): a private
//! [`TokenState`] (the user's `strava_id` plus the token fields the refresh
//! machinery drives) and a process-wide in-memory [`TOKEN_CACHE`] keyed by
//! [`UserId`].
//!
//! The DB keeps the refresh token as the durable fallback; on a cache miss the
//! [`TokenState`] is built from the DB's refresh token (an empty access token
//! with a past expiry), forcing the first request to refresh. `get_token`
//! (crate-private) is the seam the thin HTTP free functions drive: it returns a
//! valid access token plus the *new* refresh token if a refresh just happened
//! (the signal to persist). No token type or value crosses the crate boundary.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use log::debug;
use oauth2::{AccessToken, RefreshToken, TokenResponse};

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::{StravaDataStore, StravaId, StravaUser};
use tb_domain::{Error, TbResult, UserId};

/// A one-line expiry check: a known expiry in the past is expired; an unknown
/// expiry (`None`) is treated as not expired.
fn is_expired(expires_at: &Option<SystemTime>) -> bool {
    expires_at.is_some_and(|e| SystemTime::now() > e)
}

/// The token state for a single Strava user (spec #602): the user's
/// `strava_id` plus the token fields the refresh machinery drives.
///
/// Private to `tb_strava`: no token type or value crosses the crate
/// boundary. Built from the DB's durable refresh token on a cache miss (an
/// empty access token with a past expiry, so the first request forces a
/// refresh).
#[derive(Debug, Clone)]
pub(crate) struct TokenState {
    strava_id: StravaId,
    access_token: AccessToken,
    expires_at: Option<SystemTime>,
    refresh_token: Option<RefreshToken>,
}

impl TokenState {
    /// Build the token state from a stored [`StravaUser`]: an empty access
    /// token with a past expiry, so the first Strava request forces a token
    /// refresh (spec #602).
    fn from_strava_user(user: &StravaUser) -> Self {
        Self {
            strava_id: user.strava_id(),
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token: user.refresh_token(),
        }
    }

    /// A warm token state (test-only): a valid access token with a future
    /// expiry, so no refresh is needed.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn warm(strava_id: StravaId, access_token: &str) -> Self {
        Self {
            strava_id,
            access_token: AccessToken::new(access_token.to_string()),
            expires_at: Some(SystemTime::now() + std::time::Duration::from_secs(3600)),
            refresh_token: Some(RefreshToken::new("test-token".to_string())),
        }
    }

    /// The user's Strava ID.
    fn strava_id(&self) -> StravaId {
        self.strava_id
    }

    /// Ensure a valid access token, refreshing in place if expired. Returns
    /// `(access, new_refresh)` where `new_refresh` is `Some` only when a
    /// refresh issued a new rotating token (the signal to persist).
    async fn get_token(&mut self) -> TbResult<(AccessToken, Option<RefreshToken>)> {
        if !is_expired(&self.expires_at) {
            return Ok((self.access_token.clone(), None));
        }

        let current_refresh = self.refresh_token.clone().ok_or_else(|| {
            Error::NotAuth("No Refresh Token provided and Access Token expired".to_string())
        })?;
        debug!("refreshing Strava token");
        let token = STRAVACLIENT
            .exchange_refresh_token(&current_refresh)
            .request_async(&*HTTP_CLIENT)
            .await
            .map_err(|err| Error::NotAuth(err.to_string()))?;

        self.access_token = token.access_token().clone();
        self.expires_at = token.expires_in().map(|d| SystemTime::now() + d);
        let new_refresh = token.refresh_token().cloned();
        self.refresh_token = new_refresh.clone();
        Ok((self.access_token.clone(), new_refresh))
    }
}

/// The process-wide in-memory token cache (spec #602), keyed by
/// [`UserId`]. The DB keeps the refresh token as the durable fallback; on a
/// cache miss the [`TokenState`] is built from the DB's refresh token,
/// forcing a refresh.
pub(crate) static TOKEN_CACHE: LazyLock<Mutex<HashMap<UserId, TokenState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The `TokenCache` lookup (spec #602): a cache hit clones the warm
/// [`TokenState`]; a cache miss builds it from the DB's refresh token (and
/// inserts it), forcing a refresh on the first request.
///
/// The `MutexGuard` is not `Send` and must not be held across the `await`:
/// the lookup lock is scoped to its own block (cloned on a hit), and the miss
/// path reads the `StravaUser` with no lock held, then locks again to insert
/// the new state.
async fn cached_state(user_id: UserId, store: &mut impl StravaDataStore) -> TbResult<TokenState> {
    let cached = {
        let cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
        cache.get(&user_id).cloned()
    };
    match cached {
        Some(state) => Ok(state),
        None => {
            let user = StravaUser::read(user_id, store).await?;
            let state = TokenState::from_strava_user(&user);
            let mut cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
            cache.insert(user_id, state.clone());
            Ok(state)
        }
    }
}

/// A valid Strava access token for `user_id` (spec #602): a cache hit clones
/// the warm [`TokenState`]; a cache miss builds it from the DB's refresh
/// token (and inserts it), forcing a refresh. A refresh always stores the
/// durable refresh token: when the exchange issues a new rotating token it is
/// persisted to the DB via `update_token` before the access token is
/// returned, so a refresh can never happen without the rotated token being
/// stored. Returns `(strava_id, access)`.
pub(crate) async fn get_token(
    user_id: UserId,
    store: &mut impl StravaDataStore,
) -> TbResult<(StravaId, AccessToken)> {
    let mut state = cached_state(user_id, store).await?;
    let (access, new_refresh) = state.get_token().await?;
    let strava_id = state.strava_id();
    if let Some(refresh) = new_refresh {
        let secret = refresh.secret().to_string();
        strava_id.update_token(Some(secret), store).await?;
    }
    {
        let mut cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
        cache.insert(user_id, state);
    }
    Ok((strava_id, access))
}

/// The user's [`StravaId`] (spec #602): the no-refresh sibling of
/// [`get_token`]. A cache hit returns the cached [`TokenState`]'s `strava_id`;
/// a cache miss builds it from the DB's [`StravaUser`] (and inserts it). It is
/// a read-only `strava_id` resolution: it never calls `get_token` /
/// `STRAVACLIENT`. Private to `tb_strava` — no `strava_id` crosses the crate
/// boundary.
pub(crate) async fn get_strava_id(
    user_id: UserId,
    store: &mut impl StravaDataStore,
) -> TbResult<StravaId> {
    let state = cached_state(user_id, store).await?;
    Ok(state.strava_id())
}
