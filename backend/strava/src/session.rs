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
//! `get_strava`) lives in the shared [`StravaTokens`] helper (see
//! [`crate::token`]); this session is the loop-scoped driver. It embeds a
//! [`StravaTokens`] and persists a refreshed token to the DB via
//! [`DbTokenPersistence`]. The web layer's request session keeps its
//! tower-session half and its impersonation constructor for the HTTP path; the
//! two types deliberately diverge (the request session is serialized into the
//! tower session and carries `shop`; this one is in-memory and loop-scoped).

use anyhow::Context;
use log::{debug, trace};
use serde::de::DeserializeOwned;

use crate::client::{HTTP_CLIENT, STRAVACLIENT};
use crate::token::{DbTokenPersistence, StravaTokens};
use crate::{StravaDataStore, StravaId, StravaSession, StravaStore, StravaUser};
use tb_domain::{Session as TbSession, ShopId, TbResult, UserId};

/// The concrete [`StravaSession`] (spec #558 §6.2): the user's
/// identity plus the Strava API client, with the token-refresh machinery
/// the loop drives. In-memory and loop-scoped — no tower-session half.
pub struct StravaUserSession {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    tokens: StravaTokens,
}

impl StravaUserSession {
    /// Build the session for a user from their stored
    /// [`StravaUser`] (the refresh token): a cold token state (an empty
    /// access token with a past expiry), so the first Strava request forces a
    /// token refresh (spec #558 §6.2).
    pub async fn for_user(user: UserId, store: &mut impl StravaStore) -> TbResult<Self> {
        let user = StravaUser::read(user, store).await?;

        let strava_id = user.strava_id();
        let id = user.tb_id();
        let is_admin = id.is_admin(store).await?;

        let mut tokens = StravaTokens::cold();
        tokens.refresh_token = user.refresh_token();

        Ok(Self {
            id,
            strava_id,
            is_admin,
            tokens,
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
        let mut persist = DbTokenPersistence::new(self.strava_id, store);
        let r = self
            .tokens
            .get_strava(uri, &self.id, &mut persist)
            .await?
            .text()
            .await
            .context("could not reach strava")?;
        trace!("{uri}:\n{r}");
        Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
    }

    async fn deauthorize(&mut self, store: &mut impl StravaDataStore) -> TbResult<()> {
        let mut persist = DbTokenPersistence::new(self.strava_id, store);
        self.tokens.check_token(&self.id, &mut persist).await?;

        STRAVACLIENT
            .revoke_token(self.tokens.access_token.clone().into())
            .context("revoke token config error")?
            .add_extra_param("access_token", self.tokens.access_token.secret()) // Strava does not follow the standard.
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

    use crate::test_support::{TestStravaStore, strava_user};
    use tb_domain::UserStore;

    /// The loop-scoped session is built cold from the DB's refresh token: an
    /// empty, expired access token (so the first request forces a refresh)
    /// plus the stored refresh token.
    #[tokio::test]
    async fn for_user_builds_a_cold_session() -> TbResult<()> {
        let mut store = TestStravaStore::new();
        UserStore::create(&mut store.mem, "Tenda", "Bike", &None).await?;
        store.insert_user(strava_user(UserId::from(1), 42, true));

        let session = StravaUserSession::for_user(UserId::from(1), &mut store).await?;

        assert!(session.tokens.is_expired());
        assert_eq!(session.tokens.access_token.secret(), "");
        assert_eq!(
            session
                .tokens
                .refresh_token
                .as_ref()
                .map(|t| t.secret().to_string()),
            Some("test-token".to_string())
        );
        Ok(())
    }
}
