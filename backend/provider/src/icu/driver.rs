//! The intervals.icu driver type (spec #1029, ticket #1032): the
//! [`Provider`](crate::driver::Provider) impl for intervals.icu — a thin
//! forward of the driver's free functions onto the shared skeleton. The free
//! functions are unchanged; this type is the seam a future provider-agnostic
//! call site (the lane, the dispatch) drives through.
//!
//! The token model is the durable access token: `new_user` stores it as-is,
//! the remote revocation is the trait's default no-op, and the
//! [`TokenRefresh`](crate::token::TokenRefresh) impl is the trait's defaults
//! (no refresh logic, no token cache — intervals.icu has no revocation API,
//! and the cleared token is the disable signal).

use oauth2::AccessToken;

use crate::driver::Provider;
use crate::facade::{
    ActivityId, EntityFacade, Error, Part, PartId, Session, Summary, TbResult, UserId,
};
use crate::icu::{IcuId, IcuSpec, IcuUser};
use crate::store::Store;
use crate::token::TokenRefresh;

/// The intervals.icu driver type (spec #1029, ticket #1032): the
/// [`Provider`](crate::driver::Provider) impl for intervals.icu — a thin
/// forward of the driver's free functions onto the shared skeleton (the
/// author-written ops delegate to the free functions; the shared skeleton is
/// the trait's defaults, the remote revocation included — a durable-token
/// provider's no-op).
#[derive(Debug, Default, Clone, Copy)]
pub struct IcuDriver;

#[async_trait::async_trait]
impl Provider<IcuSpec> for IcuDriver {
    fn new_user(id: IcuId, tbid: UserId, token: Option<String>) -> IcuUser {
        IcuUser {
            id,
            tendabike_id: tbid,
            // The normalized optional token string builds the native typed
            // token (spec #1029's token model — symmetric with Strava's
            // typed optional refresh token).
            access_token: token.map(AccessToken::new),
        }
    }

    fn disabled(user: &IcuUser) -> bool {
        user.disabled()
    }

    fn authorize_url(state: &str, code_verifier: Option<&str>) -> String {
        use super::client::{SCOPES, icu_client};
        use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope};

        // The intervals.icu flow is PKCE (S256): the authorize URL carries the
        // driver's client id, redirect URI, and scopes, plus the S256
        // challenge derived from the code verifier.
        let mut builder = icu_client()
            .authorize_url(|| CsrfToken::new(state.to_string()))
            .add_scope(Scope::new(SCOPES.to_string()));
        if let Some(verifier) = code_verifier {
            let challenge = PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(
                verifier.to_string(),
            ));
            builder = builder.set_pkce_challenge(challenge);
        }
        let (auth_url, _) = builder.url();
        auth_url.to_string()
    }

    async fn login<S: Store<IcuSpec>, D: EntityFacade<S>>(
        code: &str,
        scope: &str,
        verifier: Option<&str>,
        facade: &D,
        store: &mut S,
    ) -> TbResult<(UserId, bool)> {
        let verifier = verifier.ok_or_else(|| {
            Error::BadRequest("missing code verifier for the icu login".to_string())
        })?;
        super::login(code, scope, verifier, facade, store).await
    }

    async fn accept_event<S: Store<IcuSpec>>(body: &str, store: &mut S) -> TbResult<Vec<UserId>> {
        super::event::accept_event(body, store).await
    }

    async fn import_gear<S: Store<IcuSpec>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Vec<PartId>> {
        super::gear::update_gear(user_id, facade, store).await
    }

    async fn process<S: Store<IcuSpec>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        super::event::process(user_id, facade, store).await
    }

    async fn activity_url<S: Store<IcuSpec>, D: EntityFacade<S>>(
        session: Box<dyn Session>,
        id: ActivityId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<String> {
        super::activity::activity_url(id, session, facade, store).await
    }

    fn gear_url(_part: &Part) -> TbResult<Option<String>> {
        // icu gear is a private settings item with no public page — the typed
        // absence.
        Ok(None)
    }

    async fn user_url<S: Store<IcuSpec>>(id: UserId, store: &mut S) -> TbResult<String> {
        super::user_url(id, store).await
    }
}

/// The durable-token provider's refresh hooks: the trait's defaults (no
/// refresh logic, no token cache).
#[async_trait::async_trait]
impl TokenRefresh<IcuSpec> for IcuDriver {}
