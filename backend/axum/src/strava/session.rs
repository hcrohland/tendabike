use anyhow::Context;
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use http::{StatusCode, request::Parts};
use log::{debug, trace};
use oauth2::TokenResponse;
#[cfg(test)]
use oauth2::{AccessToken, RefreshToken};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use std::time::SystemTime;
use time::{Duration, UtcDateTime};
use tower_sessions::Session as TowerSession;

use tb_domain::{Session as TbSession, ShopId, TbResult, UserId};
use tb_strava::client::{HTTP_CLIENT, STRAVACLIENT, StravaTokenResponse};
use tb_strava::{
    StravaDataStore, StravaId, StravaSession, StravaTokens, StravaUser, TokenPersistence,
};

/// The cookie-backed acting session (spec #590): the user's identity plus the
/// Strava token state, serialized into the tower session (the cookie). The
/// access token stays in the cookie, warm across executor reaps; a refresh
/// writes it back here via [`CookieTokenPersistence`]. The DB is never touched
/// on the web path — the refresh token remains the only credential in the
/// `stravauser` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RequestSession {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    tokens: StravaTokens,
    shop: Option<ShopId>,
    #[serde(skip)]
    session: Option<TowerSession>,
}

const SESSION_KEY: &str = "session";
const SESSION_NEXT_UPDATE: &str = "update";

impl RequestSession {
    /// Write the tower session for a just-logged-in user (spec #558 §5.2):
    /// the identity upsert is the service API's `login_from_token` op (one
    /// transaction — `StravaUser::upsert` + `is_admin`); this keeps only the
    /// tower-session half (the access token and expiry ride the session,
    /// not the DB).
    pub(crate) async fn create_from_token(
        token: StravaTokenResponse,
        session: TowerSession,
        user: StravaUser,
        is_admin: bool,
    ) -> TbResult<()> {
        trace!("got token {:?})", token);

        let tokens = StravaTokens {
            access_token: token.access_token().clone(),
            expires_at: token.expires_in().map(|d| SystemTime::now() + d),
            refresh_token: token.refresh_token().cloned(),
        };
        Self {
            id: user.tb_id(),
            strava_id: user.strava_id(),
            is_admin,
            tokens,
            session: Some(session),
            shop: None,
        }
        .update()
        .await
    }

    /// A session with plausible values for tests (valid access token for an hour).
    #[cfg(test)]
    pub(crate) fn new_dummy(is_admin: bool) -> Self {
        Self {
            id: UserId::from(1),
            strava_id: StravaId::from(1),
            is_admin,
            tokens: StravaTokens {
                access_token: AccessToken::new("test-access-token".to_owned()),
                expires_at: Some(SystemTime::now() + std::time::Duration::from_secs(3600)),
                refresh_token: Some(RefreshToken::new("test-refresh-token".to_owned())),
            },
            shop: None,
            session: None,
        }
    }

    async fn check_update(&self) -> TbResult<()> {
        if let Some(session) = self.session.clone()
            && let Some(next) = session
                .get::<UtcDateTime>(SESSION_NEXT_UPDATE)
                .await
                .context("session get")?
            && next < UtcDateTime::now()
        {
            session
                .insert(SESSION_NEXT_UPDATE, UtcDateTime::now() + Duration::days(1))
                .await
                .context("session insert")?;
        }

        Ok(())
    }

    async fn update(&self) -> TbResult<()> {
        if let Some(session) = self.session.clone() {
            trace!("updating session for user {}", self.id);
            session
                .insert(SESSION_KEY, self)
                .await
                .context("session insert failed")?;
        };
        Ok(())
    }

    fn set_session(mut self, session: TowerSession) -> Self {
        self.session = Some(session);
        self
    }

    /// Set the active shop and persist it to the tower session (spec #517,
    /// ticket #588): the web layer's cookie-persistence concern, kept out of
    /// the domain `Session` trait (which is all `&self` — the acting session
    /// is immutable to the domain).
    pub(crate) fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop;
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self.update()))
    }
}

/// The web layer's token persistence step (spec #590): after a refresh, write
/// the whole `RequestSession` (with the new access token) back to the cookie
/// via the tower-session handle. The refresh token remains the only credential
/// in the `stravauser` row — the DB is NOT touched on the web path.
///
/// Built from owned/cloned session state (the `Copy` fields plus a cloned
/// handle) so it does not borrow the `RequestSession` — whose `tokens` is
/// mutably borrowed by the token machinery while the step is live (the
/// `TokenPersistence` borrow and the `&mut self.tokens` borrow would
/// otherwise conflict).
struct CookieTokenPersistence {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    shop: Option<ShopId>,
    session: Option<TowerSession>,
}

impl CookieTokenPersistence {
    /// Build the step from the session's owned/cloned state.
    fn new(
        id: UserId,
        strava_id: StravaId,
        is_admin: bool,
        shop: Option<ShopId>,
        session: Option<TowerSession>,
    ) -> Self {
        Self {
            id,
            strava_id,
            is_admin,
            shop,
            session,
        }
    }
}

#[async_trait]
impl TokenPersistence for CookieTokenPersistence {
    async fn persist(&mut self, tokens: &StravaTokens) -> TbResult<()> {
        if let Some(session) = self.session.clone() {
            let value = RequestSession {
                id: self.id,
                strava_id: self.strava_id,
                is_admin: self.is_admin,
                tokens: tokens.clone(),
                shop: self.shop,
                session: None,
            };
            session
                .insert(SESSION_KEY, &value)
                .await
                .context("session insert failed")?;
        }
        Ok(())
    }
}

impl TbSession for RequestSession {
    fn user_id(&self) -> UserId {
        self.id
    }

    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn shop(&self) -> Option<ShopId> {
        self.shop
    }
}

#[async_trait]
impl StravaSession for RequestSession {
    fn strava_id(&self) -> StravaId {
        self.strava_id
    }

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        _store: &mut impl StravaDataStore,
    ) -> TbResult<T> {
        let mut persist = CookieTokenPersistence::new(
            self.id,
            self.strava_id,
            self.is_admin,
            self.shop,
            self.session.clone(),
        );
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

    async fn deauthorize(&mut self, _store: &mut impl StravaDataStore) -> TbResult<()> {
        let mut persist = CookieTokenPersistence::new(
            self.id,
            self.strava_id,
            self.is_admin,
            self.shop,
            self.session.clone(),
        );
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

impl<S> FromRequestParts<S> for RequestSession
where
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let towersession = TowerSession::from_request_parts(parts, state).await?;

        let Some(session) = towersession
            .get::<RequestSession>(SESSION_KEY)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Get Session"))?
        else {
            return Err((StatusCode::UNAUTHORIZED, "Please login"));
        };

        session
            .check_update()
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Update Session"))?;
        Ok(session.set_session(towersession))
    }
}

pub struct AxumAdmin;

impl<S> FromRequestParts<S> for AxumAdmin
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = RequestSession::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        if !user.is_admin() {
            Err((http::StatusCode::NOT_FOUND, "Page not found").into_response())
        } else {
            Ok(AxumAdmin)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tower_sessions::MemoryStore;

    /// A `StravaTokens` with a valid (warm) access token.
    fn warm_tokens() -> StravaTokens {
        StravaTokens {
            access_token: AccessToken::new("warm-access".to_owned()),
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
            refresh_token: Some(RefreshToken::new("warm-refresh".to_owned())),
        }
    }

    /// A `StravaTokens` with an expired access token.
    fn expired_tokens() -> StravaTokens {
        StravaTokens {
            access_token: AccessToken::new("expired-access".to_owned()),
            expires_at: Some(SystemTime::now() - Duration::from_secs(1)),
            refresh_token: Some(RefreshToken::new("expired-refresh".to_owned())),
        }
    }

    /// Write a `RequestSession` carrying `tokens` to a fresh cookie over
    /// `store`, and return the session plus its tower handle.
    async fn cookie_session(
        store: &MemoryStore,
        tokens: StravaTokens,
    ) -> (RequestSession, TowerSession) {
        let handle = TowerSession::new(None, Arc::new(store.clone()), None);
        let session = RequestSession {
            id: UserId::from(1),
            strava_id: StravaId::from(1),
            is_admin: false,
            tokens,
            shop: None,
            session: Some(handle.clone()),
        };
        handle
            .insert(SESSION_KEY, &session)
            .await
            .expect("insert session");
        (session, handle)
    }

    /// A warm token makes no refresh: `check_token` returns early, so the
    /// cookie's access token is unchanged.
    #[tokio::test]
    async fn warm_token_makes_no_refresh() -> TbResult<()> {
        let store = MemoryStore::default();
        let (mut session, handle) = cookie_session(&store, warm_tokens()).await;

        let mut persist = CookieTokenPersistence::new(
            session.id,
            session.strava_id,
            session.is_admin,
            session.shop,
            session.session.clone(),
        );
        session
            .tokens
            .check_token(&session.id, &mut persist)
            .await?;

        let stored: RequestSession = handle
            .get(SESSION_KEY)
            .await
            .expect("get session")
            .expect("session present");
        assert_eq!(stored.tokens.access_token.secret(), "warm-access");
        Ok(())
    }

    /// A refreshed token is written back to the cookie: the persistence step
    /// reconstructs the session with the new access token and writes it to the
    /// cookie (the refresh token remains the only DB credential).
    #[tokio::test]
    async fn refresh_persists_access_token_to_cookie() -> TbResult<()> {
        let store = MemoryStore::default();
        let (session, handle) = cookie_session(&store, expired_tokens()).await;

        let mut persist = CookieTokenPersistence::new(
            session.id,
            session.strava_id,
            session.is_admin,
            session.shop,
            session.session.clone(),
        );
        // The post-refresh state the token machinery hands to the step.
        let refreshed = StravaTokens {
            access_token: AccessToken::new("new-access".to_owned()),
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
            refresh_token: Some(RefreshToken::new("new-refresh".to_owned())),
        };
        persist.persist(&refreshed).await?;

        let stored: RequestSession = handle
            .get(SESSION_KEY)
            .await
            .expect("get session")
            .expect("session present");
        assert_eq!(stored.tokens.access_token.secret(), "new-access");
        assert_eq!(
            stored
                .tokens
                .refresh_token
                .as_ref()
                .map(|t| t.secret().to_string()),
            Some("new-refresh".to_string())
        );
        Ok(())
    }
}
