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

//! The shared Strava token machinery: the token state, the expiry check, the
//! refresh-token exchange, and the authenticated Strava GET.
//!
//! A session embeds a [`StravaTokens`] and drives Strava calls through it.
//! Token persistence after a refresh is pluggable via [`TokenPersistence`]:
//! the loop-scoped [`StravaUserSession`](crate::StravaUserSession) writes the
//! refreshed refresh token to the DB ([`DbTokenPersistence`]); a cookie-backed
//! session (a future driver) writes the access token to the cookie.

use std::time::SystemTime;

use anyhow::Context;
use http::StatusCode;
use log::debug;
use oauth2::{AccessToken, RefreshToken, TokenResponse};
use tb_domain::{Error, TbResult, UserId};

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::{StravaDataStore, StravaId};

const API: &str = "https://www.strava.com/api/v3";

/// The Strava token state and the machinery to keep it valid: the expiry
/// check, the refresh-token exchange, and the authenticated Strava GET.
///
/// A session owns one and drives Strava calls through it. The token fields are
/// public so a session — and a pluggable persistence step — can read them;
/// mutations go through the methods below.
#[derive(Debug, Clone)]
pub struct StravaTokens {
    /// The bearer access token.
    pub access_token: AccessToken,
    /// When the access token expires (`None` = unknown / treated as valid).
    pub expires_at: Option<SystemTime>,
    /// The refresh token (the rotating credential).
    pub refresh_token: Option<RefreshToken>,
}

impl StravaTokens {
    /// A cold token state: an empty access token with a past expiry, so the
    /// first Strava request forces a refresh. The loop-scoped
    /// [`StravaUserSession`](crate::StravaUserSession) starts here
    /// (spec #558 §6.2).
    pub fn cold() -> Self {
        Self {
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token: None,
        }
    }

    /// Whether the access token is expired.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => SystemTime::now() > expires_at,
            None => false,
        }
    }

    /// Ensure the access token is valid, refreshing it (and persisting the
    /// refreshed token) if it has expired.
    pub async fn check_token(
        &mut self,
        id: &UserId,
        persist: &mut impl TokenPersistence,
    ) -> TbResult<()> {
        if self.is_expired() {
            debug!("access token for user {} is expired", id);
            return self.refresh_the_token(id, persist).await;
        }
        Ok(())
    }

    /// Exchange the refresh token for a fresh access token, update the
    /// in-memory state, and run the persistence step with the new state.
    pub async fn refresh_the_token(
        &mut self,
        id: &UserId,
        persist: &mut impl TokenPersistence,
    ) -> TbResult<()> {
        let token = match self.refresh_token.clone() {
            Some(token) => token,
            None => {
                return Err(Error::NotAuth(
                    "No Refresh Token provided and Access Token expired".to_string(),
                ));
            }
        };
        debug!("refreshing token for user {}", id);
        let token = match STRAVACLIENT
            .exchange_refresh_token(&token)
            .request_async(&*HTTP_CLIENT)
            .await
        {
            Ok(token) => token,
            Err(err) => return Err(Error::NotAuth(err.to_string())),
        };
        self.access_token = token.access_token().clone();
        self.expires_at = token.expires_in().map(|d| SystemTime::now() + d);
        self.refresh_token = token.refresh_token().cloned();
        persist.persist(self).await?;
        Ok(())
    }

    /// The authenticated Strava GET: ensure a valid token, then GET `uri`
    /// with bearer auth, mapping the response status to a [`TbResult`].
    pub async fn get_strava(
        &mut self,
        uri: &str,
        id: &UserId,
        persist: &mut impl TokenPersistence,
    ) -> TbResult<reqwest::Response> {
        self.check_token(id, persist).await?;

        let resp = reqwest::Client::new()
            .get(format!("{API}{uri}"))
            .bearer_auth(self.access_token.secret())
            .send()
            .await
            .context("Could not reach strava")?;

        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }

        match status {
            StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT => {
                Err(Error::TryAgain(status.canonical_reason().unwrap()))
            }
            StatusCode::UNAUTHORIZED => Err(Error::NotAuth(format!(
                "Strava request authorization withdrawn for {uri}"
            ))),
            _ => Err(Error::BadRequest(format!(
                "Strava request error: {}",
                status
                    .canonical_reason()
                    .unwrap_or("Unknown status received")
            ))),
        }
    }
}

/// A pluggable persistence step for a refreshed Strava token.
///
/// The loop-scoped [`StravaUserSession`](crate::StravaUserSession) persists the
/// refreshed refresh token to the DB ([`DbTokenPersistence`]); a cookie-backed
/// session (a future driver) persists the access token to the cookie.
#[async_trait::async_trait]
pub trait TokenPersistence: Send {
    /// Persist the refreshed token state.
    async fn persist(&mut self, tokens: &StravaTokens) -> TbResult<()>;
}

/// The DB persistence step: writes the refreshed refresh token to the
/// `strava_users` row — the loop-scoped session's persistence, unchanged from
/// before the extraction.
pub struct DbTokenPersistence<'a, S: StravaDataStore> {
    strava_id: StravaId,
    store: &'a mut S,
}

impl<'a, S: StravaDataStore> DbTokenPersistence<'a, S> {
    /// Build the DB persistence step for `strava_id` over `store`.
    pub fn new(strava_id: StravaId, store: &'a mut S) -> Self {
        Self { strava_id, store }
    }
}

#[async_trait::async_trait]
impl<S: StravaDataStore> TokenPersistence for DbTokenPersistence<'_, S> {
    async fn persist(&mut self, tokens: &StravaTokens) -> TbResult<()> {
        let refresh = tokens.refresh_token.as_ref().map(RefreshToken::secret);
        self.strava_id.update_token(refresh, self.store).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::test_support::{TestStravaStore, strava_user};

    fn tokens(expires_at: Option<SystemTime>) -> StravaTokens {
        StravaTokens {
            access_token: AccessToken::new("test-access-token".to_owned()),
            expires_at,
            refresh_token: Some(RefreshToken::new("test-refresh-token".to_owned())),
        }
    }

    #[test]
    fn expired_when_access_token_expired() {
        assert!(tokens(Some(SystemTime::now() - Duration::from_secs(1))).is_expired());
    }

    #[test]
    fn not_expired_when_access_token_valid() {
        assert!(!tokens(Some(SystemTime::now() + Duration::from_secs(3600))).is_expired());
    }

    #[test]
    fn not_expired_when_expiry_unknown() {
        assert!(!tokens(None).is_expired());
    }

    /// The pluggable DB persistence step writes the refreshed refresh token to
    /// the `strava_users` row (the loop-scoped session's persistence).
    #[tokio::test]
    async fn db_persistence_writes_refresh_token() -> TbResult<()> {
        let mut store = TestStravaStore::new();
        store.insert_user(strava_user(UserId::from(1), 42, true));

        let tokens = StravaTokens {
            access_token: AccessToken::new("new-access".into()),
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
            refresh_token: Some(RefreshToken::new("new-refresh".into())),
        };

        let mut persist = DbTokenPersistence::new(StravaId::from(42), &mut store);
        persist.persist(&tokens).await?;

        let stored = store
            .stravauser_get_by_stravaid(&StravaId::from(42))
            .await?
            .and_then(|u| u.refresh_token().map(|t| t.secret().to_string()));
        assert_eq!(stored, Some("new-refresh".to_string()));
        Ok(())
    }
}
