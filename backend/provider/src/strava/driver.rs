//! The Strava driver type (spec #1029, ticket #1032): the
//! [`Provider`](crate::driver::Provider) impl for Strava — a thin forward of
//! the driver's free functions onto the shared skeleton. The free functions
//! are unchanged; this type is the seam a future provider-agnostic call site
//! (the lane, the dispatch) drives through.

use crate::driver::Provider;
use crate::facade::{ActivityId, EntityFacade, PartId, Session, Summary, TbResult, UserId};
use crate::store::Store;
use crate::strava::{StravaId, StravaSpec, StravaUser};
use oauth2::RefreshToken;

/// The Strava driver type (spec #1029, ticket #1032): the
/// [`Provider`](crate::driver::Provider) impl for Strava — a thin forward of
/// the driver's free functions onto the shared skeleton (the author-written
/// ops delegate to the free functions; the shared skeleton is the trait's
/// defaults).
#[derive(Debug, Default, Clone, Copy)]
pub struct StravaDriver;

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
        super::http::deauthorize(id, store).await
    }

    fn authorize_url(state: &str, code_verifier: Option<&str>) -> String {
        crate::Provider::Strava.authorize_url(state, code_verifier)
    }

    async fn login<S: Store<StravaSpec>, D: EntityFacade<S>>(
        code: &str,
        scope: &str,
        _verifier: Option<&str>,
        facade: &D,
        store: &mut S,
    ) -> TbResult<(UserId, bool)> {
        super::login(code, scope, facade, store).await
    }

    async fn accept_event<S: Store<StravaSpec>>(
        body: &str,
        store: &mut S,
    ) -> TbResult<Vec<UserId>> {
        let queued = super::event::accept_event(body, store).await?;
        Ok(queued.into_iter().collect())
    }

    async fn import_gear<S: Store<StravaSpec>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Vec<PartId>> {
        StravaUser::update_gear(user_id, facade, store).await
    }

    async fn process<S: Store<StravaSpec>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        super::event::process(user_id, facade, store).await
    }

    async fn activity_url<S: Store<StravaSpec>, D: EntityFacade<S>>(
        session: Box<dyn Session>,
        id: ActivityId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<String> {
        super::activity::strava_url(id, session, facade, store).await
    }

    async fn gear_url<S: Store<StravaSpec>, D: EntityFacade<S>>(
        session: Box<dyn Session>,
        id: PartId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Option<String>> {
        let part = facade.part(id, session, store).await?;
        super::gear::strava_url(&part)
    }

    async fn user_url<S: Store<StravaSpec>>(id: UserId, store: &mut S) -> TbResult<String> {
        super::strava_url(id, store).await
    }
}
