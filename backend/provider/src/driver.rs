//! The provider driver trait (spec #1029, ticket #1032; consolidated in spec
//! #1062): the single driver interface, generic over the
//! [`ProviderSpec`](crate::ProviderSpec) marker.
//!
//! [`Provider`] is the driver a provider author implements. It splits into
//! the genuinely provider-specific operations the author writes (the OAuth
//! login, the webhook intake, the gear import, the processing loop body, the
//! activity/gear/user URL operations, the authorize URL, the token hooks —
//! the token response, the shared oauth2 client, and the refresh-token
//! accessor — and the row hooks) and the shared user-lifecycle skeleton the
//! author gets free as default methods (upsert, deauthorize, delete,
//! all-stats, onboard, insert-sync, and the token `refresh` — the kit's
//! refresh exchange over the provider's shared client).
//!
//! The split rule: a method is on the trait only if it has a genuinely
//! provider-specific core the author must write (an OAuth exchange, a wire
//! parse, an API call, a URL base, the row/token shape). A generic operation
//! on the store's user/event concept is a default method (provided) or a
//! [`Store`](crate::store::Store) operation. Concrete-provider knowledge is
//! type-level (on the marker); generic behavior is on the trait.
//!
//! The token concern sits in one place on the trait, beside the other
//! row/token hooks (spec #1062): the refresh exchange is the kit's free
//! function (the [`refresh`](Provider::refresh) default), and a durable-token
//! provider omits the override entirely — its token path reads the durable
//! token directly from the user row and never reaches the token cache's
//! refresh flow.
//!
//! The trait is namespaced under `driver` (not re-exported at the crate root)
//! because the crate root's [`Provider`](crate::Provider) is the provider
//! enum — the same namespacing the
//! [`dispatch`](crate::dispatch) module keeps for its generic op names.

use log::{debug, warn};
use oauth2::RefreshToken;

use crate::ProviderSpec;
use crate::client::OauthClient;
use crate::facade::{
    ActivityId, EntityFacade, Error, Part, PartId, Session, Summary, TbResult, UserId, UserStat,
};
use crate::store::Store;

/// The refreshed-token value the driver trait's
/// [`refresh`](Provider::refresh) returns (re-exported: it is named in the
/// public trait signature and lives in the crate-internal `token` module).
pub use crate::token::RefreshedToken;

/// The provider driver (spec #1029, consolidated in spec #1062): the single
/// driver interface, generic over the [`ProviderSpec`](crate::ProviderSpec)
/// marker.
///
/// The author writes the provider-specific core: the OAuth login, the webhook
/// intake, the gear import, the processing loop body, the activity/gear/user
/// URL operations, the authorize URL, the token hooks (the token response,
/// the shared oauth2 client, and the refresh-token accessor — a default
/// absence for a provider whose tokens are durable), and the row hooks
/// (construct a user row from a normalized optional token, the disabled
/// check, and the remote token revocation — a default no-op for a provider
/// whose tokens are durable).
///
/// The author gets the shared user-lifecycle skeleton free as default
/// methods: `upsert`, `user_deauthorize`, `user_delete`, `all_stats`,
/// `onboard`, `insert_sync` (the marker's `make_sync` composed with the
/// store's `event_create`), and `refresh` (the kit's refresh exchange over
/// the provider's shared client — a provider whose tokens are durable
/// omits the override entirely; its token path never reaches the exchange).
#[async_trait::async_trait]
pub trait Provider<P: ProviderSpec>: Send + Sync {
    // ── the author writes: the token hooks ──────────────────────────────────

    /// The provider's token response type: the wire shape of the oauth2
    /// token response (the standard response plus the provider's extra
    /// fields). `Send` (the `refresh` default's `async_trait` `Send` bound
    /// holds the response across the exchange's await).
    type TokenResponse: oauth2::TokenResponse + Send;

    /// The provider's oauth2 client: the full client shape the refresh
    /// exchange and the authorize URL need — the shared client module's
    /// [`OauthClient`] helper trait (the common provider client is a type
    /// alias, which cannot appear in a trait bound, so the shape is named
    /// by the trait). `'static` (the client is a process-wide static — the
    /// shared reference outlives every call).
    type Client: OauthClient<TokenResponse = Self::TokenResponse> + 'static;

    /// The provider's shared oauth2 client (a static reference: built once
    /// from the provider config, shared by every provider call — the same
    /// shape as the rate-limit handle and the token cache).
    fn client() -> &'static Self::Client;

    /// Refresh the user's access token (spec #1056): the kit's refresh
    /// exchange over the provider's shared client — the standard oauth2
    /// refresh-token call, written once in the kit. Takes the optional
    /// refresh token: an absent token is the exchange's `NotAuth` (the
    /// single home of the no-refresh-token rule).
    ///
    /// Default: the exchange (no provider re-implements it). A provider
    /// whose tokens are durable (intervals.icu) does not override this: its
    /// token path reads the durable token directly from the user row and
    /// never reaches the token cache's refresh flow, so its inherited
    /// exchange is never called.
    async fn refresh(refresh_token: Option<RefreshToken>) -> TbResult<RefreshedToken> {
        crate::token::refresh_exchange(Self::client(), refresh_token).await
    }

    /// Read the refresh token out of the user row (spec #1056) — the durable
    /// fallback the token cache's miss builds from.
    ///
    /// Default: `None` (a provider whose tokens are durable has no refresh
    /// token).
    fn refresh_token(_user: &P::User) -> Option<RefreshToken> {
        None
    }

    // ── the author writes: the row hooks ────────────────────────────────────

    /// Construct the provider's user row from the provider id, the
    /// TendaBike user id, and the normalized optional token (the provider's
    /// wire-shape concern: the author builds the native typed token).
    fn new_user(id: P::Owner, tbid: UserId, token: Option<String>) -> P::User;

    /// Whether the user's provider integration is disabled (the cleared
    /// token is the disable signal).
    fn disabled(user: &P::User) -> bool;

    /// Revoke the user's remote provider token (the deauthorization's remote
    /// half). Default: a no-op (a provider whose tokens are durable has no
    /// revocation API).
    async fn revoke<S: Store<P>>(_id: UserId, _store: &mut S) -> TbResult<()> {
        Ok(())
    }

    /// The OAuth authorize URL (the driver's client id, redirect URI, and
    /// scopes, plus the CSRF state the web shell generates). `code_verifier`
    /// is the opaque PKCE code verifier (a PKCE flow derives its S256
    /// challenge from it; a plain authorization-code exchange ignores it).
    fn authorize_url(state: &str, code_verifier: Option<&str>) -> String;

    // ── the author writes: the provider-specific core ──────────────────────

    /// The OAuth login: the scope check, the token exchange, the
    /// wire-shape parsing of the athlete, and the upsert, returning the
    /// identity and the admin flag. `code`, `scope`, and `verifier` are the
    /// opaque callback values (no driver wire type crosses the seam).
    async fn login<S: Store<P>, D: EntityFacade<S>>(
        code: &str,
        scope: &str,
        verifier: Option<&str>,
        facade: &D,
        store: &mut S,
    ) -> TbResult<(UserId, bool)>;

    /// The webhook intake: the envelope parse of the raw body and the queue
    /// insert, returning the TendaBike users a row was queued for (the
    /// caller's wake list).
    async fn accept_event<S: Store<P>>(body: &str, store: &mut S) -> TbResult<Vec<UserId>>;

    /// Import the user's gear (bikes + shoes), returning the created part ids.
    async fn import_gear<S: Store<P>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Vec<PartId>>;

    /// Process the user's next queued event (the lane's `process`): one event
    /// per call, the queue's read and failure policy the kit's.
    async fn process<S: Store<P>, D: EntityFacade<S>>(
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary>;

    /// The provider's URL for the activity `id`, gated on the acting
    /// session's ownership (the read is the ownership check).
    async fn activity_url<S: Store<P>, D: EntityFacade<S>>(
        session: Box<dyn Session>,
        id: ActivityId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<String>;

    /// The provider's gear page for the part `part` (already read by the
    /// caller — the part read is the ownership check). The one non-total op:
    /// a provider without a public gear page is the typed absence (`None`).
    fn gear_url(part: &Part) -> TbResult<Option<String>>;

    /// The provider's public user profile URL for the TendaBike user `id`
    /// (no session: the route has no user context to gate on).
    async fn user_url<S: Store<P>>(id: UserId, store: &mut S) -> TbResult<String>;

    // ── the author gets free: the shared skeleton ──────────────────────────

    /// Upsert the provider user by provider id: update the domain user and
    /// the token for an existing connection, or create the domain user plus
    /// the provider user row (via [`new_user`](Self::new_user)) for a new one.
    async fn upsert<S: Store<P>, D: EntityFacade<S>>(
        id: P::Owner,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        token: Option<String>,
        facade: &D,
        store: &mut S,
    ) -> TbResult<P::User> {
        let user = store.user_get_by_provider_id(&id).await?;
        if let Some(user) = user {
            facade
                .update_user(P::user_tbid(&user), firstname, lastname, avatar, store)
                .await?;
            return store.user_update_token(id, token).await;
        }

        // create new user!
        let tbid = facade
            .create_user(firstname, lastname, avatar, store)
            .await?;
        let user = Self::new_user(id, tbid, token);
        store.user_create(user).await
    }

    /// Deauthorize the user (admin-only): revoke their remote token (the
    /// provider's [`revoke`](Self::revoke) — a failed revocation is
    /// non-fatal, logged) and disable their local row (their queued events
    /// dropped, their token cleared).
    async fn user_deauthorize<S: Store<P>>(
        session: &mut dyn Session,
        tbid: UserId,
        store: &mut S,
    ) -> TbResult<()> {
        if !session.is_admin() {
            return Err(Error::Forbidden(
                "only an admin can deauthorize a user".to_string(),
            ));
        }

        if let Err(err) = Self::revoke(tbid, store).await {
            warn!("could not deauthorize user {tbid}: {err:#}")
        }

        warn!("User {tbid} deauthorized");

        let user = store.user_get_by_tbid(tbid).await?;
        let owner = P::user_owner(&user).clone();
        store.user_disable(owner).await
    }

    /// Delete the user (admin-only): deauthorize them plus delete their
    /// provider and domain rows.
    async fn user_delete<S: Store<P>, D: EntityFacade<S>>(
        session: &mut dyn Session,
        tbid: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<()> {
        if !session.is_admin() {
            return Err(Error::Forbidden(
                "only an admin can delete a user".to_string(),
            ));
        }
        debug!("Deauthorizing user {tbid}");
        Self::user_deauthorize(session, tbid, store).await?;
        let n = store.user_delete(tbid).await?;
        debug!("Deleted {n} user");
        facade.delete_user(&tbid, store).await
    }

    /// The all-user stat read: every provider user's provider-neutral
    /// [`UserStat`] — the domain stat plus the provider's queued-event count
    /// and disabled flag, stamped with the provider's wire value.
    async fn all_stats<S: Store<P>, D: EntityFacade<S>>(
        facade: &D,
        store: &mut S,
    ) -> TbResult<Vec<UserStat>> {
        let users = store.user_get_all().await?;

        let mut res = Vec::new();
        for u in users {
            let events = store.user_event_count(P::user_owner(&u)).await?;
            res.push(UserStat {
                stat: facade.get_stat(P::user_tbid(&u), store).await?,
                events,
                disabled: Self::disabled(&u),
                provider: P::PROVIDER.wire_value().to_string(),
            });
        }
        Ok(res)
    }

    /// The per-user onboarding: import the user's gear (bikes + shoes), then
    /// enqueue the durable activity-sync event, returning the created gear
    /// part ids (the onboarding response lists them).
    async fn onboard<S: Store<P>, D: EntityFacade<S>>(
        user_id: UserId,
        time: i64,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Vec<PartId>> {
        let gear = Self::import_gear(user_id, facade, store).await?;
        Self::insert_sync(user_id, time, store).await?;
        Ok(gear)
    }

    /// Enqueue the user's durable activity-sync event: the marker's
    /// [`make_sync`](ProviderSpec::make_sync) composed with the store's
    /// [`event_create`](Store::event_create).
    async fn insert_sync<S: Store<P>>(user_id: UserId, time: i64, store: &mut S) -> TbResult<()> {
        let user = store.user_get_by_tbid(user_id).await?;
        let owner = P::user_owner(&user).clone();
        store.event_create(P::make_sync(owner, time)).await
    }
}

#[cfg(test)]
mod driver_tests;
