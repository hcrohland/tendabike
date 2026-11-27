//! The Strava driver type (spec #1029, ticket #1032): the
//! [`Provider`](crate::driver::Provider) impl for Strava — a thin forward of
//! the driver's free functions onto the shared skeleton. The free functions
//! are unchanged; this type is the seam a future provider-agnostic call site
//! (the lane, the dispatch) drives through.

use crate::RefreshedToken;
use crate::driver::Provider;
use crate::facade::{ActivityId, EntityFacade, Part, PartId, Session, Summary, TbResult, UserId};
use crate::store::Store;
use crate::strava::{StravaId, StravaSpec, StravaUser};
use crate::token::TokenRefresh;
use oauth2::RefreshToken;

/// The Strava driver type (spec #1029, ticket #1032): the
/// [`Provider`](crate::driver::Provider) impl for Strava — a thin forward of
/// the driver's free functions onto the shared skeleton (the author-written
/// ops delegate to the free functions; the shared skeleton is the trait's
/// defaults).
#[derive(Debug, Default, Clone, Copy)]
pub struct StravaDriver;

impl StravaDriver {
    /// The Strava gear URL for a part: a `b…` source is a bike page, a `g…`
    /// source is the gear settings page; a typed absence (`None`) — the route
    /// maps it to a no-op; an unrecognized source prefix is a `NotFound`.
    pub(crate) fn gear_url_from_part(
        part: &crate::facade::Part,
    ) -> crate::facade::TbResult<Option<String>> {
        use crate::facade::Error;
        let Some(g) = &part.source else {
            return Ok(None);
        };
        match &g[0..1] {
            "b" => Ok(Some(format!("https://strava.com/bikes/{}", &g[1..]))),
            "g" => Ok(Some("https://www.strava.com/settings/gear".into())),
            _ => Err(Error::NotFound("".to_string())),
        }
    }
}

#[async_trait::async_trait]
impl Provider<StravaSpec> for StravaDriver {
    fn new_user(id: StravaId, tbid: UserId, token: Option<String>) -> StravaUser {
        StravaUser {
            id,
            tendabike_id: tbid,
            refresh_token: token.map(RefreshToken::new),
        }
    }

    fn disabled(user: &StravaUser) -> bool {
        user.disabled()
    }

    async fn revoke<S: Store<StravaSpec>>(id: UserId, store: &mut S) -> TbResult<()> {
        use super::client::strava_client;
        use super::http::access_token;
        use crate::client::HTTP_CLIENT;
        use anyhow::Context;
        use log::debug;

        let (strava_id, access) = access_token(id, store).await?;

        // The offline seam (test-only): in test mode a deauthorization is
        // recorded in the thread-local map instead of hitting the Strava API.
        #[cfg(any(test, feature = "test-support"))]
        if crate::test_support::in_test_mode() {
            super::test_support::record_deauthorize(strava_id);
            debug!("user {} token revoked (test)", strava_id);
            return Ok(());
        }

        strava_client()
            .revoke_token(access.clone().into())
            .context("revoke token config error")?
            .add_extra_param("access_token", access.secret()) // Strava does not follow the standard.
            .request_async(&*HTTP_CLIENT)
            .await
            .context("token exchange failed")?;
        debug!("user {} token revoked", strava_id);
        Ok(())
    }

    fn authorize_url(state: &str, _code_verifier: Option<&str>) -> String {
        use super::client::{SCOPES, strava_client};
        use oauth2::{CsrfToken, Scope};

        // The Strava flow is a plain authorization-code exchange: no PKCE
        // challenge, so the code verifier is unused. The URL carries the
        // driver's client id, redirect URI, and scopes.
        let (auth_url, _) = strava_client()
            .authorize_url(|| CsrfToken::new(state.to_string()))
            .add_scope(Scope::new(SCOPES.to_string()))
            .url();
        auth_url.to_string()
    }

    async fn login<S: Store<StravaSpec>, D: EntityFacade<S>>(
        code: &str,
        scope: &str,
        _verifier: Option<&str>,
        facade: &D,
        store: &mut S,
    ) -> TbResult<(UserId, bool)> {
        super::user::do_login(code, scope, facade, store).await
    }

    async fn accept_event<S: Store<StravaSpec>>(
        body: &str,
        store: &mut S,
    ) -> TbResult<Vec<UserId>> {
        use super::event::InEvent;
        use crate::facade::Error;
        use log::trace;

        let event: InEvent = serde_json::from_str(body)
            .map_err(|e| Error::BadRequest(format!("invalid strava webhook envelope: {e}")))?;
        trace!("Received {event:#?}");
        let queued = event.accept(store).await?;
        Ok(queued.into_iter().collect())
    }

    async fn import_gear<S: Store<StravaSpec>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Vec<PartId>> {
        use super::gear::into_partid;
        use super::http::request_json;
        use serde::Deserialize;

        if super::ratelimit::limited_until().is_some() {
            log::debug!("update_gear: rate limited, skipping hydration");
            return Ok(Vec::new());
        }

        #[derive(Deserialize, Debug)]
        struct Gear {
            id: String,
        }

        #[derive(Deserialize, Debug)]
        struct Athlete {
            bikes: Vec<Gear>,
            shoes: Vec<Gear>,
        }

        let ath: Athlete = request_json(user_id, "/athlete", store).await?;

        let mut parts = Vec::new();
        for gear in ath.bikes.into_iter().chain(ath.shoes) {
            parts.push(into_partid(gear.id, user_id, facade, store).await?);
        }

        Ok(parts)
    }

    async fn process<S: Store<StravaSpec>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        use super::event::ObjectType;
        use super::session::get_owner_id;
        use crate::lane::QueueRead;
        use crate::queue::get_event;
        use log::{info, warn};

        let strava_id = get_owner_id(user_id, store).await?;
        let read = get_event(strava_id, store).await?;
        let event = match read {
            QueueRead::Event(event) => event,
            // The queue is empty, or paused by a global rate-limit `Stop` (the
            // caller sleeps until its deadline; nothing to process).
            QueueRead::Limited { .. } | QueueRead::Empty => return Ok(Summary::default()),
        };
        info!("Processing {event}");

        match event.object_type {
            ObjectType::Activity => event.process_activity(user_id, facade, store).await,
            ObjectType::Sync => event.process_sync(user_id, facade, store).await,
            _ => {
                warn!("skipping {event}");
                event.delete(store).await?;
                Ok(Summary::default())
            }
        }
    }

    async fn activity_url<S: Store<StravaSpec>, D: EntityFacade<S>>(
        session: Box<dyn Session>,
        id: ActivityId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<String> {
        use crate::ProviderActivityId;
        use crate::facade::Error;

        let ProviderActivityId::Strava(raw) = ProviderActivityId::try_from(id)? else {
            return Err(Error::BadRequest("not a strava activity".to_string()));
        };
        let full: ActivityId = ProviderActivityId::Strava(raw).into();
        facade.read_activity(&full, session, store).await?;
        Ok(format!("https://strava.com/activities/{raw}"))
    }

    fn gear_url(part: &Part) -> TbResult<Option<String>> {
        Self::gear_url_from_part(part)
    }

    async fn user_url<S: Store<StravaSpec>>(id: UserId, store: &mut S) -> TbResult<String> {
        let user = store.user_get_by_tbid(id).await?;
        Ok(format!("https://strava.com/athletes/{}", user.strava_id()))
    }
}

#[async_trait::async_trait]
impl TokenRefresh<StravaSpec> for StravaDriver {
    async fn refresh(refresh_token: String) -> TbResult<RefreshedToken> {
        use super::client::strava_client;
        use crate::client::HTTP_CLIENT;
        use crate::facade::Error;
        use oauth2::TokenResponse;

        let current = RefreshToken::new(refresh_token);
        let token = strava_client()
            .exchange_refresh_token(&current)
            .request_async(&*HTTP_CLIENT)
            .await
            .map_err(|err| Error::NotAuth(err.to_string()))?;

        Ok(RefreshedToken {
            access_token: token.access_token().clone(),
            expires_in: token.expires_in(),
            refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
        })
    }

    fn refresh_token(user: &StravaUser) -> Option<String> {
        user.refresh_token().map(|t| t.secret().to_string())
    }
}
