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

use std::time::SystemTime;

use log::debug;
use oauth2::{AccessToken, RefreshToken, TokenResponse};

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::{StravaId, StravaSession, StravaStore, StravaUser};
use tb_domain::{Error, Session as TbSession, ShopId, TbResult, UserId};

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
        let id = user.tb_id();
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

    /// A one-line inline expiry check: a known expiry in the past is expired;
    /// an unknown expiry (`None`) is treated as not expired.
    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|e| SystemTime::now() > e)
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
        if !self.is_expired() {
            return Ok((self.access_token.clone(), None));
        }

        let refresh_token = self.refresh_token.clone().ok_or_else(|| {
            Error::NotAuth("No Refresh Token provided and Access Token expired".to_string())
        })?;
        debug!("refreshing token for user {}", self.id);
        let token = STRAVACLIENT
            .exchange_refresh_token(&refresh_token)
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
        assert!(dummy(Some(SystemTime::now() - Duration::from_secs(1))).is_expired());
    }

    #[test]
    fn not_expired_when_access_token_valid() {
        assert!(!dummy(Some(SystemTime::now() + Duration::from_secs(3600))).is_expired());
    }

    #[test]
    fn not_expired_when_expiry_unknown() {
        assert!(!dummy(None).is_expired());
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
}
