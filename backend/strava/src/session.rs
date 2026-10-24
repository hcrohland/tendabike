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

//! The concrete [`StravaSession`] the executor's admin-sync path builds
//! (spec #558 §6.2).
//!
//! In-memory and loop-scoped — no tower-session half. The token state
//! (`access_token`/`expires_at`/`refresh_token`) is plain fields; the refresh
//! machinery lives entirely in [`get_token`](StravaUserSession::get_token).
//! The web layer's request session keeps its tower-session half and its
//! impersonation constructor for the HTTP path; the two types deliberately
//! diverge (the request session is serialized into the tower session and
//! carries `shop`; this one is in-memory and loop-scoped).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use log::debug;
use oauth2::{AccessToken, RefreshToken, TokenResponse};

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::{StravaDataStore, StravaId, StravaSession, StravaStore, StravaUser};
use tb_domain::{Error, Session as TbSession, ShopId, TbResult, UserId};

/// A one-line expiry check: a known expiry in the past is expired; an unknown
/// expiry (`None`) is treated as not expired.
pub fn is_expired(expires_at: &Option<SystemTime>) -> bool {
    expires_at.is_some_and(|e| SystemTime::now() > e)
}

/// The shared token-refresh machinery (the single seam both `get_token` impls
/// drive): if the access token is expired, exchange the refresh token for a
/// fresh pair in place; otherwise return the warm access token. Returns
/// `(access, new_refresh)` where `new_refresh` is `Some` only when a refresh
/// issued a new rotating token (the signal to persist).
pub async fn refresh_if_expired(
    access_token: &mut AccessToken,
    expires_at: &mut Option<SystemTime>,
    refresh_token: &mut Option<RefreshToken>,
) -> TbResult<(AccessToken, Option<RefreshToken>)> {
    if !is_expired(expires_at) {
        return Ok((access_token.clone(), None));
    }

    let current_refresh = refresh_token.clone().ok_or_else(|| {
        Error::NotAuth("No Refresh Token provided and Access Token expired".to_string())
    })?;
    debug!("refreshing Strava token");
    let token = STRAVACLIENT
        .exchange_refresh_token(&current_refresh)
        .request_async(&*HTTP_CLIENT)
        .await
        .map_err(|err| Error::NotAuth(err.to_string()))?;

    *access_token = token.access_token().clone();
    *expires_at = token.expires_in().map(|d| SystemTime::now() + d);
    let new_refresh = token.refresh_token().cloned();
    *refresh_token = new_refresh.clone();
    Ok((access_token.clone(), new_refresh))
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
        refresh_if_expired(
            &mut self.access_token,
            &mut self.expires_at,
            &mut self.refresh_token,
        )
        .await
    }
}

/// The process-wide in-memory token cache (spec #602), keyed by
/// [`UserId`]. The DB keeps the refresh token as the durable fallback; on a
/// cache miss the [`TokenState`] is built from the DB's refresh token,
/// forcing a refresh.
pub(crate) static TOKEN_CACHE: LazyLock<Mutex<HashMap<UserId, TokenState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// A valid Strava access token for `user_id` (spec #602): a cache hit clones
/// the warm [`TokenState`]; a cache miss builds it from the DB's refresh
/// token (and inserts it), forcing a refresh. Returns `(strava_id, access,
/// new_refresh)` where `new_refresh` is `Some` only when a refresh issued a
/// new rotating token (the signal to persist).
pub async fn get_token(
    user_id: UserId,
    store: &mut impl StravaDataStore,
) -> TbResult<(StravaId, AccessToken, Option<RefreshToken>)> {
    // The `MutexGuard` is not `Send` and must not be held across the `await`:
    // the lookup lock is scoped to its own block (cloned on a hit), and the
    // miss path reads the StravaUser with no lock held, then locks again to
    // insert the new state.
    let mut state = {
        let cached = {
            let cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
            cache.get(&user_id).cloned()
        };
        match cached {
            Some(state) => state,
            None => {
                let user = StravaUser::read(user_id, store).await?;
                let state = TokenState::from_strava_user(&user);
                let mut cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
                cache.insert(user_id, state.clone());
                state
            }
        }
    };
    let (access, new_refresh) = state.get_token().await?;
    let strava_id = state.strava_id();
    {
        let mut cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
        cache.insert(user_id, state);
    }
    Ok((strava_id, access, new_refresh))
}

/// The user's [`StravaId`] (spec #602): the no-refresh sibling of
/// [`get_token`]. A cache hit returns the cached [`TokenState`]'s `strava_id`;
/// a cache miss builds it from the DB's [`StravaUser`] (and inserts it). It is
/// a read-only `strava_id` resolution: it never calls `get_token` /
/// `refresh_if_expired` / `STRAVACLIENT`. Private to `tb_strava` — no
/// `strava_id` crosses the crate boundary.
pub(crate) async fn get_strava_id(
    user_id: UserId,
    store: &mut impl StravaDataStore,
) -> TbResult<StravaId> {
    let state = {
        let cached = {
            let cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
            cache.get(&user_id).cloned()
        };
        match cached {
            Some(state) => state,
            None => {
                let user = StravaUser::read(user_id, store).await?;
                let state = TokenState::from_strava_user(&user);
                let mut cache = TOKEN_CACHE.lock().expect("the token cache is not poisoned");
                cache.insert(user_id, state.clone());
                state
            }
        }
    };
    Ok(state.strava_id())
}

/// The concrete [`StravaSession`] (spec #558 §6.2): the user's identity plus
/// the Strava token state, with the refresh machinery the loop drives.
/// In-memory and loop-scoped — no tower-session half.
pub struct StravaUserSession {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    access_token: AccessToken,
    expires_at: Option<SystemTime>,
    refresh_token: Option<RefreshToken>,
}

impl StravaUserSession {
    /// Build the session for a user from their stored
    /// [`StravaUser`] (the refresh token): an empty access token with a
    /// past expiry, so the first Strava request forces a token refresh
    /// (spec #558 §6.2).
    pub async fn for_user(user: UserId, store: &mut impl StravaStore) -> TbResult<Self> {
        let user = StravaUser::read(user, store).await?;

        let strava_id = user.strava_id();
        let id = user.user_id();
        let is_admin = id.is_admin(store).await?;
        let refresh_token = user.refresh_token();

        Ok(Self {
            id,
            strava_id,
            is_admin,
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token,
        })
    }
}

impl TbSession for StravaUserSession {
    fn user_id(&self) -> UserId {
        self.id
    }

    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn shop(&self) -> Option<ShopId> {
        None
    }
}

#[async_trait::async_trait]
impl StravaSession for StravaUserSession {
    fn strava_id(&self) -> StravaId {
        self.strava_id
    }

    /// Ensure a valid access token, refreshing in place if expired.
    ///
    /// A warm session returns `(access, None)` (no refresh call). An expired
    /// session makes exactly one `STRAVACLIENT` exchange and returns
    /// `(new_access, Some(new_refresh))`. The refresh token is *not* persisted
    /// here — the caller (the thin free functions) persists it to the DB when
    /// the returned `Option` is `Some`.
    async fn get_token(&mut self) -> TbResult<(AccessToken, Option<RefreshToken>)> {
        debug!("get_token for user {}", self.id);
        refresh_if_expired(
            &mut self.access_token,
            &mut self.expires_at,
            &mut self.refresh_token,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn dummy(expires_at: Option<SystemTime>) -> StravaUserSession {
        StravaUserSession {
            id: UserId::from(1),
            strava_id: StravaId::from(1),
            is_admin: true,
            access_token: AccessToken::new("test-access-token".to_owned()),
            expires_at,
            refresh_token: Some(RefreshToken::new("test-refresh-token".to_owned())),
        }
    }

    #[test]
    fn expired_when_access_token_expired() {
        assert!(is_expired(&Some(
            SystemTime::now() - Duration::from_secs(1)
        )));
    }

    #[test]
    fn not_expired_when_access_token_valid() {
        assert!(!is_expired(&Some(
            SystemTime::now() + Duration::from_secs(3600)
        )));
    }

    #[test]
    fn not_expired_when_expiry_unknown() {
        assert!(!is_expired(&None));
    }

    /// A warm `get_token` makes no refresh call: it returns the stored access
    /// token and `None` (no rotated refresh token).
    #[tokio::test]
    async fn get_token_warm_returns_the_access_token_and_no_refresh() -> TbResult<()> {
        let mut session = dummy(Some(SystemTime::now() + Duration::from_secs(3600)));
        let (access, new_refresh) = session.get_token().await?;
        assert_eq!(access.secret(), "test-access-token");
        assert!(new_refresh.is_none());
        Ok(())
    }

    /// An expired `get_token` with no refresh token fails with `NotAuth` (the
    /// refresh is impossible) — no network call is made.
    #[tokio::test]
    async fn get_token_expired_without_a_refresh_token_fails() {
        let mut session = StravaUserSession {
            id: UserId::from(1),
            strava_id: StravaId::from(1),
            is_admin: true,
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token: None,
        };
        let res = session.get_token().await;
        assert!(matches!(res, Err(Error::NotAuth(_))));
    }

    // ─── Tests for the shared `refresh_if_expired` helper ───────────────

    /// A warm `refresh_if_expired` (not expired) returns the stored access
    /// token and `None` (no refresh call, no rotated refresh token).
    #[tokio::test]
    async fn refresh_if_expired_warm_returns_the_access_token_and_no_refresh() {
        let mut access = AccessToken::new("warm-access".to_owned());
        let mut expires_at = Some(SystemTime::now() + Duration::from_secs(3600));
        let mut refresh = Some(RefreshToken::new("warm-refresh".to_owned()));

        let (returned_access, new_refresh) =
            refresh_if_expired(&mut access, &mut expires_at, &mut refresh)
                .await
                .expect("warm session succeeds");

        assert_eq!(returned_access.secret(), "warm-access");
        assert!(new_refresh.is_none());
        // The fields are untouched: no refresh happened.
        assert_eq!(access.secret(), "warm-access");
        assert!(expires_at.is_some());
        assert_eq!(refresh.as_ref().unwrap().secret(), "warm-refresh");
    }

    /// An expired `refresh_if_expired` with no refresh token fails with
    /// `NotAuth` (the refresh is impossible) — no network call is made.
    #[tokio::test]
    async fn refresh_if_expired_expired_without_a_refresh_token_fails() {
        let mut access = AccessToken::new(String::default());
        let mut expires_at = Some(SystemTime::UNIX_EPOCH);
        let mut refresh: Option<RefreshToken> = None;

        let res = refresh_if_expired(&mut access, &mut expires_at, &mut refresh).await;
        assert!(matches!(res, Err(Error::NotAuth(_))));
    }

    // ─── Tests for the shared `is_expired` helper ────────────────────────

    #[test]
    fn is_expired_true_when_expiry_in_the_past() {
        assert!(is_expired(&Some(
            SystemTime::now() - Duration::from_secs(1)
        )));
    }

    #[test]
    fn is_expired_false_when_expiry_in_the_future() {
        assert!(!is_expired(&Some(
            SystemTime::now() + Duration::from_secs(3600)
        )));
    }

    #[test]
    fn is_expired_false_when_expiry_unknown() {
        assert!(!is_expired(&None));
    }
}
