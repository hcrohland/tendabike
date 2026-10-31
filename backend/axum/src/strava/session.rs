use anyhow::Context;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use http::{StatusCode, request::Parts};
use log::trace;
use serde_derive::{Deserialize, Serialize};
use time::{Duration, UtcDateTime};
use tower_sessions::Session as TowerSession;

use tb_domain::{Session as TbSession, ShopId, TbResult, UserId};

/// The acting session (ADR-0009): an identity half and a context half. The
/// identity (`id`, `is_admin`) is per-browser — it lives in the tower-session
/// cookie, shared by every tab. The context (`shop`) is per-request: it rides
/// each request's `X-Active-Shop` header (the SSE stream's `?shop=` is the
/// same value on the stream route) and is never persisted in the cookie, so
/// the server is stateless with respect to it and tabs cannot interfere.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RequestSession {
    id: UserId,
    is_admin: bool,
    /// The per-request active shop (the context half, ADR-0009): read from
    /// the `X-Active-Shop` header at extraction, never written to the cookie.
    #[serde(skip)]
    shop: Option<ShopId>,
    #[serde(skip)]
    session: Option<TowerSession>,
}

/// The fetch requests' per-request active-shop header (ADR-0009). The SSE
/// stream keeps its `?shop=` query parameter — the native `EventSource`
/// client cannot send custom headers.
pub(crate) const ACTIVE_SHOP_HEADER: &str = "X-Active-Shop";

const SESSION_KEY: &str = "session";
const SESSION_NEXT_UPDATE: &str = "update";

impl RequestSession {
    /// Write the tower session for a just-logged-in user (spec #558 §5.2):
    /// the identity upsert is the service API's `login_from_token` op (one
    /// transaction — `StravaUser::upsert` + `is_admin`); this keeps only the
    /// tower-session half (the token flows through the `tb_strava`
    /// `TokenCache`, not the cookie).
    pub(crate) async fn create_from_token(
        user_id: UserId,
        is_admin: bool,
        session: TowerSession,
    ) -> TbResult<()> {
        Self {
            id: user_id,
            is_admin,
            session: Some(session),
            shop: None,
        }
        .update()
        .await
    }

    /// A session with plausible values for tests.
    #[cfg(test)]
    pub(crate) fn new_dummy(is_admin: bool) -> Self {
        Self {
            id: UserId::from(1),
            is_admin,
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

    /// Re-read the current session from the tower session cookie: the fresh
    /// [`RequestSession`] the stream's re-subscribe hands the loop (warm across
    /// reaps — the cookie carries the latest token state after a refresh).
    pub(crate) async fn read_from_cookie(&self) -> TbResult<RequestSession> {
        let session = self.session.clone().ok_or_else(|| {
            tb_domain::Error::AnyFailure(anyhow::anyhow!("the tower-session handle is missing"))
        })?;
        let Ok(Some(fresh)) = session
            .get::<RequestSession>(SESSION_KEY)
            .await
            .map_err(|err| {
                tb_domain::Error::AnyFailure(anyhow::anyhow!("reading the session: {err}"))
            })
        else {
            return Err(tb_domain::Error::NotAuth(
                "no session in the cookie".to_string(),
            ));
        };
        Ok(fresh)
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
        // The context half (ADR-0009): the per-request active shop rides the
        // `X-Active-Shop` header (a malformed value is no shop).
        let mut session = session.set_session(towersession);
        if let Some(shop) = parts
            .headers
            .get(ACTIVE_SHOP_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i32>().ok())
            .map(ShopId::from)
        {
            session.shop = Some(shop);
        }
        Ok(session)
    }
}

/// The admin-verification extractor: a `RequestSession` that has been
/// checked `is_admin()`. It carries the acting session through, so the admin
/// handlers can hand it to the registry (which verifies admin in a later
/// `tb_strava` ticket) rather than rebuilding an identity-only one.
pub struct AxumAdmin {
    session: RequestSession,
}

impl AxumAdmin {
    /// The admin's acting session.
    pub(crate) fn session(&self) -> RequestSession {
        self.session.clone()
    }
}

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
            Ok(AxumAdmin { session: user })
        }
    }
}
