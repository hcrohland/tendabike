use std::sync::OnceLock;

use super::Provider;
use crate::ProviderSpec;
use crate::client::ProviderClient;
use crate::facade::{
    ActivityId, EntityFacade, MockEntityFacade, OnboardingStatus, Part, PartId, Session, Stat,
    Summary, TbResult, User, UserId, mock_session, mock_session_admin,
};
use crate::store::Store;
use crate::strava::client::StravaTokenResponse;
use crate::strava::{StravaId, StravaSpec, StravaUser};
use crate::test_support::MockStore;
use oauth2::{AuthUrl, ClientId, EndpointNotSet, EndpointSet, RefreshToken, TokenUrl};

/// The test user row for [`StravaSpec`].
fn user(tbid: i32, stravaid: i64, enabled: bool) -> StravaUser {
    StravaUser {
        id: StravaId::from(stravaid),
        tendabike_id: UserId::from(tbid),
        refresh_token: enabled.then(|| RefreshToken::new("refresh".to_string())),
    }
}

/// A canned driver over [`StravaSpec`]: the author-written ops are inlined
/// (canned outcomes, no HTTP), so the shared skeleton's default methods run
/// for real over the single [`MockStore`] double (spec #1029, ticket #1032).
struct TestDriver;

/// The canned driver's dummy client's type: the common provider client over
/// the Strava token response, with the endpoint markers left generic (so
/// `::new` — defined only on the all-unset client — is resolved through the
/// un-instantiated markers, and the setter chain flips them).
type DummyClient<
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointSet,
> = ProviderClient<
    StravaTokenResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

/// The canned driver's dummy oauth client (locally-pointing auth/token
/// URLs): the token hooks' `Client` associated type — the skeleton tests
/// never refresh, so the client is never called (spec #1062).
fn dummy_client() -> &'static DummyClient {
    static CLIENT: OnceLock<DummyClient> = OnceLock::new();
    CLIENT.get_or_init(|| {
        DummyClient::new(ClientId::new("test".to_string()))
            .set_auth_uri(AuthUrl::new("http://localhost/oauth/authorize".to_string()).unwrap())
            .set_token_uri(TokenUrl::new("http://localhost/oauth/token".to_string()).unwrap())
    })
}

#[async_trait::async_trait]
impl Provider<StravaSpec> for TestDriver {
    type TokenResponse = StravaTokenResponse;
    type Client = DummyClient;

    fn client() -> &'static Self::Client {
        dummy_client()
    }

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

    fn authorize_url(_state: &str, _code_verifier: Option<&str>) -> String {
        "authorize".to_string()
    }

    async fn login<S: Store<StravaSpec>, D: EntityFacade<S>>(
        _code: &str,
        _scope: &str,
        _verifier: Option<&str>,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<(UserId, bool)> {
        Ok((UserId::from(1), false))
    }

    async fn accept_event<S: Store<StravaSpec>>(
        _body: &str,
        _store: &mut S,
    ) -> TbResult<Vec<UserId>> {
        Ok(Vec::new())
    }

    async fn import_gear<S: Store<StravaSpec>, D: EntityFacade<S>>(
        _user_id: UserId,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<Vec<PartId>> {
        Ok(vec![PartId::from(7)])
    }

    async fn process<S: Store<StravaSpec>, D: EntityFacade<S>>(
        _user_id: UserId,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<Summary> {
        Ok(Summary::default())
    }

    async fn activity_url<S: Store<StravaSpec>, D: EntityFacade<S>>(
        _session: Box<dyn Session>,
        _id: ActivityId,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<String> {
        Ok("activity-url".to_string())
    }

    fn gear_url(_part: &Part) -> TbResult<Option<String>> {
        Ok(None)
    }

    async fn user_url<S: Store<StravaSpec>>(_id: UserId, _store: &mut S) -> TbResult<String> {
        Ok("user-url".to_string())
    }
}

// ─── upsert ─────────────────────────────────────────────────────────────────

/// A new provider user: no row by provider id → the domain user is created,
/// the row is built by `new_user` (the normalized token) and stored.
#[tokio::test]
async fn upsert_creates_new_user() -> TbResult<()> {
    let mut store = MockStore::<StravaSpec>::new();
    store
        .expect_user_get_by_provider_id()
        .returning(|_id| Ok(None));
    let mut facade = MockEntityFacade::new();
    facade
        .expect_create_user()
        .returning(|_f, _l, _a, _s| Ok(UserId::from(10)));
    store.expect_user_create().returning(Ok);

    let user = TestDriver::upsert(
        StravaId::from(42),
        "First",
        "Last",
        &None,
        Some("token".to_string()),
        &facade,
        &mut store,
    )
    .await?;

    assert_eq!(user.id, StravaId::from(42));
    assert_eq!(user.tendabike_id, UserId::from(10));
    assert_eq!(
        user.refresh_token.map(|t| t.secret().to_string()),
        Some("token".to_string())
    );
    Ok(())
}

/// An existing provider user: the domain user is updated and the token is
/// set (the stored row is returned).
#[tokio::test]
async fn upsert_updates_existing_user() -> TbResult<()> {
    let mut store = MockStore::<StravaSpec>::new();
    let existing = user(10, 42, true);
    store
        .expect_user_get_by_provider_id()
        .returning(move |_id| Ok(Some(existing.clone())));
    store.expect_user_update_token().returning(|id, token| {
        Ok(StravaUser {
            id,
            tendabike_id: UserId::from(10),
            refresh_token: token.map(RefreshToken::new),
        })
    });
    let mut facade = MockEntityFacade::new();
    facade
        .expect_update_user()
        .returning(|_id, _f, _l, _a, _s| Ok(UserId::from(10)));

    let user = TestDriver::upsert(
        StravaId::from(42),
        "First",
        "Last",
        &None,
        None,
        &facade,
        &mut store,
    )
    .await?;

    assert_eq!(user.tendabike_id, UserId::from(10));
    assert!(user.refresh_token.is_none());
    Ok(())
}

// ─── deauthorize / delete ───────────────────────────────────────────────────

/// Deauthorize is admin-only: a non-admin session is a `Forbidden`.
#[tokio::test]
async fn user_deauthorize_requires_admin() {
    let mut session = mock_session(UserId::from(1));
    let mut store = MockStore::<StravaSpec>::new();
    let res = TestDriver::user_deauthorize(&mut session, UserId::from(10), &mut store).await;
    assert!(matches!(res, Err(crate::facade::Error::Forbidden(_))));
}

/// Deauthorize (admin): the remote revocation is the default no-op, and the
/// local row is disabled (their queued events dropped, their token cleared).
#[tokio::test]
async fn user_deauthorize_disables_local_row() -> TbResult<()> {
    let mut session = mock_session_admin(UserId::from(1));
    let mut store = MockStore::<StravaSpec>::new();
    store
        .expect_user_get_by_tbid()
        .returning(|_id| Ok(user(10, 42, true)));
    store.expect_user_disable().times(1).returning(|owner| {
        assert_eq!(owner, StravaId::from(42));
        Ok(())
    });

    TestDriver::user_deauthorize(&mut session, UserId::from(10), &mut store).await?;
    Ok(())
}

/// Delete (admin): deauthorize plus the provider row and the domain user are
/// deleted.
#[tokio::test]
async fn user_delete_deauthorizes_and_deletes() -> TbResult<()> {
    let mut session = mock_session_admin(UserId::from(1));
    let mut store = MockStore::<StravaSpec>::new();
    store
        .expect_user_get_by_tbid()
        .returning(|_id| Ok(user(10, 42, true)));
    store.expect_user_disable().returning(|_owner| Ok(()));
    store.expect_user_delete().times(1).returning(|_id| Ok(1));
    let mut facade = MockEntityFacade::new();
    facade
        .expect_delete_user()
        .times(1)
        .returning(|_id, _s| Ok(()));

    TestDriver::user_delete(&mut session, UserId::from(10), &facade, &mut store).await?;
    Ok(())
}

// ─── all_stats ──────────────────────────────────────────────────────────────

/// All stats: every user's domain stat plus the queued-event count and the
/// disabled flag, stamped with the marker's provider wire value.
#[tokio::test]
async fn all_stats_maps_users() -> TbResult<()> {
    let mut store = MockStore::<StravaSpec>::new();
    store
        .expect_user_get_all()
        .returning(|| Ok(vec![user(1, 42, true), user(2, 43, false)]));
    store.expect_user_event_count().returning(|_owner| Ok(3));
    let mut facade = MockEntityFacade::new();
    facade.expect_get_stat().returning(|id, _s| {
        Ok(Stat::new(User {
            id: *id,
            name: "n".to_string(),
            firstname: "f".to_string(),
            avatar: None,
            is_admin: false,
            onboarding_status: OnboardingStatus::default(),
        }))
    });

    let stats = TestDriver::all_stats(&facade, &mut store).await?;

    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].events, 3);
    assert!(!stats[0].disabled);
    assert!(stats[1].disabled);
    assert_eq!(stats[0].provider, "strava");
    Ok(())
}

// ─── onboard / insert_sync ──────────────────────────────────────────────────

/// Onboard: the gear import (the driver's) plus the sync enqueue (the
/// default), returning the created gear part ids.
#[tokio::test]
async fn onboard_imports_gear_and_queues_sync() -> TbResult<()> {
    let mut store = MockStore::<StravaSpec>::new();
    store
        .expect_user_get_by_tbid()
        .returning(|_id| Ok(user(10, 42, true)));
    store.expect_event_create().times(1).returning(|_e| Ok(()));
    let facade = MockEntityFacade::new();

    let gear = TestDriver::onboard(UserId::from(10), 1_700_000_000, &facade, &mut store).await?;

    assert_eq!(gear, vec![PartId::from(7)]);
    Ok(())
}

/// Insert-sync: the marker's `make_sync` composed with the store's
/// `event_create` — the queued row is the durable sync for the user's owner.
#[tokio::test]
async fn insert_sync_composes_make_sync_and_event_create() -> TbResult<()> {
    let mut store = MockStore::<StravaSpec>::new();
    store
        .expect_user_get_by_tbid()
        .returning(|_id| Ok(user(10, 42, true)));
    store.expect_event_create().times(1).returning(|e| {
        assert_eq!(e, StravaSpec::make_sync(StravaId::from(42), 1_700_000_000));
        Ok(())
    });

    TestDriver::insert_sync(UserId::from(10), 1_700_000_000, &mut store).await?;
    Ok(())
}
