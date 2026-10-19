use anyhow::Context;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use http::{StatusCode, request::Parts};
use log::trace;
use oauth2::{AccessToken, RefreshToken, TokenResponse};
use serde_derive::{Deserialize, Serialize};
use std::time::SystemTime;
use time::{Duration, UtcDateTime};
use tower_sessions::Session as TowerSession;

use tb_domain::{Session as TbSession, ShopId, TbResult, UserId};
use tb_strava::client::StravaTokenResponse;
use tb_strava::{StravaId, StravaStore, StravaUser};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RequestSession {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    access_token: AccessToken,
    expires_at: Option<SystemTime>,
    refresh_token: Option<RefreshToken>,
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

        let refresh_token = token.refresh_token();
        Self {
            id: user.tb_id(),
            strava_id: user.strava_id(),
            is_admin,
            access_token: token.access_token().clone(),
            expires_at: token.expires_in().map(|d| SystemTime::now() + d),
            refresh_token: refresh_token.cloned(),
            session: Some(session),
            shop: None,
        }
        .update()
        .await
    }

    /// User for admin actions to impersonate user. The body is the
    /// executor-side twin's (`tb_strava::StravaUserSession::for_user`)
    /// six-field init list — the types deliberately diverge (spec #558
    /// §6.5): this one is serialized into the tower session and carries
    /// `shop`; the twin is in-memory and executor-scoped. An empty access
    /// token with a past expiry, so the first Strava call forces a token
    /// refresh. Kept as the web layer's impersonation constructor even
    /// though the direct ops build their session in-txn via the twin.
    #[allow(dead_code)]
    pub(crate) async fn create_from_id(
        _admin: AxumAdmin,
        user: UserId,
        store: &mut impl StravaStore,
    ) -> TbResult<RequestSession> {
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
            session: None,
            shop: None,
        })
    }

    /// A session with plausible values for tests (valid access token for an hour).
    #[cfg(test)]
    pub(crate) fn new_dummy(is_admin: bool) -> Self {
        Self {
            id: UserId::from(1),
            strava_id: StravaId::from(1),
            is_admin,
            access_token: AccessToken::new("test-access-token".to_owned()),
            expires_at: Some(SystemTime::now() + std::time::Duration::from_secs(3600)),
            refresh_token: Some(RefreshToken::new("test-refresh-token".to_owned())),
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

    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop;
        // Keep the trait dyn compatible
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self.update()))
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
