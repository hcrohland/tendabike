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

//! The concrete [`StravaSession`].
//!
//! The token-refresh machinery (`check_token`/`refresh_the_token`/
//! `get_strava`/`request_json`/`deauthorize`) depends only on
//! `&mut impl StravaDataStore` plus the Strava client. The web layer's
//! request session keeps its tower-session half and its impersonation
//! constructor for the HTTP path; the two types deliberately diverge (the
//! request session is serialized into the tower session and carries
//! `shop`; this one is in-memory and loop-scoped).

use std::time::SystemTime;

use anyhow::Context;
use http::StatusCode;
use log::{debug, trace};
use oauth2::{AccessToken, RefreshToken, TokenResponse};
use serde::de::DeserializeOwned;

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::{StravaDataStore, StravaId, StravaSession, StravaStore, StravaUser};
use tb_domain::{Error, Session as TbSession, ShopId, TbResult, UserId};

const API: &str = "https://www.strava.com/api/v3";

/// The concrete [`StravaSession`] (spec #558 §6.2): the user's
/// identity plus the Strava API client, with the token-refresh machinery
/// the loop drives. In-memory and loop-scoped — no tower-session half.
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

    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => SystemTime::now() > expires_at,
            None => false,
        }
    }

    async fn refresh_the_token(&mut self, store: &mut impl StravaDataStore) -> TbResult<()> {
        let token = match self.refresh_token.clone() {
            Some(token) => token,
            None => {
                return Err(Error::NotAuth(
                    "No Refresh Token provided and Access Token expired".to_string(),
                ));
            }
        };
        debug!("refreshing token for user {}", self.id);
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
        let refresh = token.refresh_token().map(|t| t.secret());
        self.strava_id.update_token(refresh, store).await?;
        Ok(())
    }

    async fn get_strava(
        &mut self,
        uri: &str,
        store: &mut impl StravaDataStore,
    ) -> TbResult<reqwest::Response> {
        self.check_token(store).await?;

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

    async fn check_token(&mut self, store: &mut impl StravaDataStore) -> TbResult<()> {
        if self.is_expired() {
            debug!("access token for user {} is expired", self.id);
            return self.refresh_the_token(store).await;
        }
        Ok(())
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

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        store: &mut impl StravaDataStore,
    ) -> TbResult<T> {
        // The suite's offline seam: a canned response stands in for the
        // Strava API (see `crate::test_support::CANNED_STRAVA`).
        #[cfg(feature = "test-support")]
        if let Some(json) = crate::test_support::canned_response(uri) {
            return Ok(serde_json::from_str(&json).context("Could not parse response body")?);
        }
        let r = self
            .get_strava(uri, store)
            .await?
            .text()
            .await
            .context("could not reach strava")?;
        trace!("{uri}:\n{r}");
        Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
    }

    async fn deauthorize(&mut self, store: &mut impl StravaDataStore) -> TbResult<()> {
        self.check_token(store).await?;

        STRAVACLIENT
            .revoke_token(self.access_token.clone().into())
            .context("revoke token config error")?
            .add_extra_param("access_token", self.access_token.secret()) // Strava does not follow the standard.
            .request_async(&*HTTP_CLIENT)
            .await
            .context("token exchange failed")?;
        debug!("user {} token revoked", self.strava_id);
        Ok(())
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
}
