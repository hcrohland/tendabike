//! The dispatch suite (ADR-0014): the `login` and `accept_event` dispatch
//! functions and their match arms, against the store mocks.
//!
//! The dispatch functions are bounded on the two providers' local data
//! stores (`StravaDataStore + IcuDataStore`), so the suite's store double
//! composes the two providers' data-store automocks — delegating field by
//! field. Domain work goes through the entity-op facade (a
//! [`MockEntityFacade`]); the arms' work runs for real against the canned
//! store outcomes. The login's token exchange is the one piece that needs
//! the live provider API, so the login tests cover the arms' scope check
//! (which precedes it).

use std::sync::{Arc, Mutex, Once};

use crate::Provider;
use crate::facade::{Error, TbResult, UserId, mock_session_admin};
use crate::icu::test_support::{MockIcuDataStore, begin_test, queue};
use crate::icu::{IcuDataStore, IcuEvent, IcuId, IcuUser};
use crate::strava::event::Event;
use crate::strava::test_support::MockStravaDataStore;
use crate::strava::{StravaDataStore, StravaId, StravaUser};
use crate::test_support::MockEntityFacade;

use super::{accept_event, deauthorize, delete_user, login, sync_users};

// ─── The composite store double ─────────────────────────────────────────────
//
// The dispatch's `StravaDataStore + IcuDataStore` bound: the two providers'
// data stores, delegated field by field. The mocks carry the canned outcomes
// the tests assert on.

/// The dispatch suite's store double: [`MockStravaDataStore`] plus
/// [`MockIcuDataStore`], delegated field by field — the
/// `StravaDataStore + IcuDataStore` bound the dispatch functions carry.
#[derive(Default)]
struct DispatchStore {
    strava: MockStravaDataStore,
    icu: MockIcuDataStore,
}

#[async_trait::async_trait]
impl StravaDataStore for DispatchStore {
    async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        self.strava.stravauser_get_all().await
    }
    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        self.strava.stravauser_get_by_tbid(id).await
    }
    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        self.strava.stravauser_get_by_stravaid(id).await
    }
    async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        self.strava.stravauser_create(user).await
    }
    async fn stravauser_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<String>,
    ) -> TbResult<StravaUser> {
        self.strava.stravauser_update_token(stravaid, refresh).await
    }
    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.strava.stravauser_delete(user).await
    }
    async fn strava_event_create(&mut self, e: Event) -> TbResult<()> {
        self.strava.strava_event_create(e).await
    }
    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.strava.strava_event_delete(event_id).await
    }
    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        self.strava.strava_event_set_time(e_id, e_time).await
    }
    async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>> {
        self.strava.strava_event_get_next_for_user(user).await
    }
    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        self.strava.strava_event_get_later(obj_id, oid).await
    }
    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.strava.strava_events_delete_batch(values).await
    }
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64> {
        self.strava.strava_events_get_count_for_user(user).await
    }
    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        self.strava.strava_events_delete_for_user(user).await
    }
}

#[async_trait::async_trait]
impl IcuDataStore for DispatchStore {
    async fn icuser_get_all(&mut self) -> TbResult<Vec<IcuUser>> {
        self.icu.icuser_get_all().await
    }
    async fn icuser_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser> {
        self.icu.icuser_get_by_tbid(id).await
    }
    async fn icuser_get_by_icuid(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>> {
        self.icu.icuser_get_by_icuid(id).await
    }
    async fn icuser_create(&mut self, user: IcuUser) -> TbResult<IcuUser> {
        self.icu.icuser_create(user).await
    }
    async fn icuser_update_token(
        &mut self,
        icuid: IcuId,
        access_token: Option<String>,
    ) -> TbResult<IcuUser> {
        self.icu.icuser_update_token(icuid, access_token).await
    }
    async fn icuser_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.icu.icuser_delete(user).await
    }
    async fn icu_event_create(&mut self, e: IcuEvent) -> TbResult<()> {
        self.icu.icu_event_create(e).await
    }
    async fn icu_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.icu.icu_event_delete(event_id).await
    }
    async fn icu_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        self.icu.icu_event_set_time(e_id, e_time).await
    }
    async fn icu_event_get_next_for_user(&mut self, user: IcuId) -> TbResult<Option<IcuEvent>> {
        self.icu.icu_event_get_next_for_user(user).await
    }
    async fn icu_events_get_count_for_user(&mut self, user: &IcuId) -> TbResult<i64> {
        self.icu.icu_events_get_count_for_user(user).await
    }
    async fn icu_events_get_latest_time(&mut self, user: &IcuId) -> TbResult<Option<i64>> {
        self.icu.icu_events_get_latest_time(user).await
    }
    async fn icu_events_delete_for_user(&mut self, user: &IcuId) -> TbResult<usize> {
        self.icu.icu_events_delete_for_user(user).await
    }
}

// ─── Fixtures ───────────────────────────────────────────────────────────────

/// A raw Strava webhook body: an activity-create event for owner `owner`.
fn strava_activity_body(owner: i32) -> String {
    format!(
        r#"{{"object_type":"activity","object_id":11,"aspect_type":"create","updates":{{}},"owner_id":{owner},"subscription_id":1,"event_time":1767348000}}"#
    )
}

/// A raw Strava webhook body: an athlete deauthorization for owner `owner`.
fn strava_deauth_body(owner: i32) -> String {
    format!(
        r#"{{"object_type":"athlete","object_id":{owner},"aspect_type":"update","updates":{{"authorized":"false"}},"owner_id":{owner},"subscription_id":1,"event_time":1767348000}}"#
    )
}

/// A raw intervals.icu webhook body with the given `secret` and events JSON
/// array elements.
fn icu_body(secret: &str, events: &str) -> String {
    format!(r#"{{"secret":"{secret}","events":[{events}]}}"#)
}

/// One intervals.icu webhook event element.
fn icu_event_json(athlete: &str, typ: &str, activity: &str, timestamp: &str) -> String {
    format!(
        r#"{{"athlete_id":"{athlete}","type":"{typ}","timestamp":"{timestamp}","activity":{{"id":"{activity}"}}}}"#
    )
}

/// One intervals.icu webhook event element with **no** `activity` field (the
/// test delivery: the app's shared secret but no activity object).
fn icu_event_json_no_activity(athlete: &str, typ: &str, timestamp: &str) -> String {
    format!("{{\"athlete_id\":\"{athlete}\",\"type\":\"{typ}\",\"timestamp\":\"{timestamp}\"}}")
}

/// Initialize the provider config once for the dispatch test process, with
/// a known webhook secret the intake tests verify against. Returns the
/// secret value.
fn icu_webhook_secret() -> String {
    static ONCE: Once = Once::new();
    const SECRET: &str = "icu-test-webhook-secret";
    ONCE.call_once(|| {
        crate::init(crate::ProviderConfig {
            strava_client_id: "test-client-id".to_string(),
            strava_client_secret: "test-client-secret".to_string(),
            strava_redirect_url: "http://localhost:8000/strava/token".to_string(),
            icu_client_id: "test-icu-client-id".to_string(),
            icu_client_secret: "test-icu-client-secret".to_string(),
            icu_redirect_url: "http://localhost:8000/icu/token".to_string(),
            icu_webhook_secret: SECRET.to_string(),
        });
    });
    SECRET.to_string()
}

// ─── accept_event: the Strava arm ───────────────────────────────────────────

/// The Strava arm parses the raw body and queues the event (the `strava_events`
/// row), returning the owner's TendaBike id (the caller's wake).
#[tokio::test]
async fn accept_event_strava_queues_an_activity_event() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let user = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_stravauser_get_by_stravaid()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(None));
    let created_cell = created.clone();
    store
        .strava
        .expect_strava_event_create()
        .returning(move |e| {
            *created_cell.lock().expect("the capture is not poisoned") = Some(e.clone());
            Ok(())
        });

    let queued = accept_event(Provider::Strava, &strava_activity_body(42), &mut store).await?;

    assert_eq!(queued, vec![UserId::from(7)]);
    let event = created
        .lock()
        .expect("the capture is not poisoned")
        .clone()
        .expect("the event was queued");
    assert_eq!(event.object_id, 11);
    assert_eq!(event.event_time, 1767348000);
    assert_eq!(event.owner_id, StravaId::from(42));
    Ok(())
}

/// The Strava arm's athlete-deauth branch: the owner is disabled (their queued
/// events dropped, their token cleared), nothing is queued, no wake.
#[tokio::test]
async fn accept_event_strava_athlete_deauth_disables_the_user() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let user = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    let lookup = user.clone();
    store
        .strava
        .expect_stravauser_get_by_stravaid()
        .returning(move |_id| Ok(Some(lookup.clone())));
    store
        .strava
        .expect_strava_events_delete_for_user()
        .returning(|_id| Ok(0));
    store
        .strava
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            assert!(refresh.is_none(), "the deauth clears the token");
            Ok(user.clone())
        });

    let queued = accept_event(Provider::Strava, &strava_deauth_body(42), &mut store).await?;

    assert!(queued.is_empty(), "a deauth queues nothing (no wake)");
    Ok(())
}

/// The Strava arm rejects an unknown owner (the `strava_events` key would have
/// no user) — a `BadRequest`, nothing queued.
#[tokio::test]
async fn accept_event_strava_unknown_owner_is_rejected() -> TbResult<()> {
    let mut store = DispatchStore::default();
    store
        .strava
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(None));

    let err = accept_event(Provider::Strava, &strava_activity_body(42), &mut store)
        .await
        .expect_err("an unknown owner is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

/// The Strava arm rejects a malformed envelope (the parse failure is a
/// `BadRequest` before any store work).
#[tokio::test]
async fn accept_event_strava_malformed_body_is_rejected() -> TbResult<()> {
    let mut store = DispatchStore::default();

    let err = accept_event(Provider::Strava, "not a json body", &mut store)
        .await
        .expect_err("a malformed body is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

// ─── accept_event: the icu arm ──────────────────────────────────────────────

/// The icu arm verifies the secret, filters to the subscribed activity types,
/// parses the timestamp, and queues one row per subscribed event — returning
/// the users a row was queued for (the caller's wake list).
#[tokio::test]
async fn accept_event_icu_queues_subscribed_events_only() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let user = crate::icu::test_support::icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_icuser_get_by_icuid()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_cell = created.clone();
    store.icu.expect_icu_event_create().returning(move |e| {
        created_cell
            .lock()
            .expect("the capture is not poisoned")
            .push(e.clone());
        Ok(())
    });

    let body = icu_body(
        &secret,
        &format!(
            "{},{}",
            icu_event_json(
                "a1",
                "ACTIVITY_UPLOADED",
                "act-1",
                "2024-12-06T06:40:47.011+00:00"
            ),
            icu_event_json(
                "a1",
                "ACTIVITY_HEART_RATE",
                "act-2",
                "2024-12-06T06:40:47.011+00:00"
            )
        ),
    );
    let queued = accept_event(Provider::Icu, &body, &mut store).await?;

    // Only the subscribed event was queued (the other type is not subscribed).
    assert_eq!(queued, vec![UserId::from(9)]);
    let created = created.lock().expect("the capture is not poisoned");
    assert_eq!(created.len(), 1);
    let event = &created[0];
    assert_eq!(event.icu_athlete_id, IcuId::from("a1"));
    assert_eq!(event.activity_id, "act-1");
    assert_eq!(event.event_time, 1733467247);
    Ok(())
}

/// The icu arm rejects a bad secret (a `NotAuth`) before any store work.
#[tokio::test]
async fn accept_event_icu_rejects_a_bad_secret() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let body = icu_body(
        "a-different-secret",
        &icu_event_json(
            "a1",
            "ACTIVITY_UPLOADED",
            "act-1",
            "2024-12-06T06:40:47.011+00:00",
        ),
    );

    let err = accept_event(Provider::Icu, &body, &mut store)
        .await
        .expect_err("a bad secret is rejected");
    assert!(matches!(err, Error::NotAuth(_)));
    Ok(())
}

/// The icu arm rejects a malformed envelope (the parse failure is a
/// `BadRequest` before any store work).
#[tokio::test]
async fn accept_event_icu_malformed_body_is_rejected() -> TbResult<()> {
    let mut store = DispatchStore::default();

    let err = accept_event(Provider::Icu, "not a json body", &mut store)
        .await
        .expect_err("a malformed body is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

/// The icu arm rejects an invalid timestamp (a `BadRequest`) once the secret
/// is verified.
#[tokio::test]
async fn accept_event_icu_rejects_an_invalid_timestamp() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let body = icu_body(
        &secret,
        &icu_event_json("a1", "ACTIVITY_UPLOADED", "act-1", "not-a-timestamp"),
    );

    let err = accept_event(Provider::Icu, &body, &mut store)
        .await
        .expect_err("an invalid timestamp is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

/// The icu arm accepts a **test delivery** (a valid secret, a `TEST` event
/// that carries **no** `activity`) with nothing queued — the event is not
/// queueable, so it is dropped (and info-logged) and intervals.icu does not
/// retry it.
#[tokio::test]
async fn accept_event_icu_test_delivery_is_accepted_and_queues_nothing() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let body = icu_body(
        &secret,
        &icu_event_json_no_activity("a1", "TEST", "2024-12-06T06:40:47.011+00:00"),
    );

    let queued = accept_event(Provider::Icu, &body, &mut store).await?;

    assert!(
        queued.is_empty(),
        "a test delivery queues nothing (no wake)"
    );
    Ok(())
}

/// The icu arm skips a **non-subscribed** event (even one that carries an
/// `activity`) with nothing queued — only the subscribed activity types are
/// imported, so the event is skipped (and debug-logged).
#[tokio::test]
async fn accept_event_icu_non_subscribed_event_is_skipped() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let body = icu_body(
        &secret,
        &icu_event_json(
            "a1",
            "ACTIVITY_HEART_RATE",
            "act-1",
            "2024-12-06T06:40:47.011+00:00",
        ),
    );

    let queued = accept_event(Provider::Icu, &body, &mut store).await?;

    assert!(
        queued.is_empty(),
        "a non-analyzed event queues nothing (no wake)"
    );
    Ok(())
}

// ─── deauthorize / delete_user: the icu arm ─────────────────────────────────

/// The icu deauthorize arm disables the user (their queued events dropped,
/// their durable token cleared) — the admin "disable user" op that used to
/// `todo!()`-panic for icu users.
#[tokio::test]
async fn deauthorize_icu_disables_the_user() -> TbResult<()> {
    let mut store = DispatchStore::default();
    // The provider lookup: no Strava row, an icu row → the icu arm.
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = crate::icu::test_support::icu_user(UserId::from(9), "a1", true);
    let icu_read = icu.clone();
    store
        .icu
        .expect_icuser_get_by_tbid()
        .returning(move |_| Ok(icu_read.clone()));
    store
        .icu
        .expect_icu_events_delete_for_user()
        .returning(|_id| Ok(0));
    let icu_token = icu.clone();
    store
        .icu
        .expect_icuser_update_token()
        .returning(move |_id, token| {
            assert!(token.is_none(), "the deauth clears the token");
            Ok(icu_token.clone())
        });

    let mut session = mock_session_admin(UserId::from(1));
    deauthorize(&mut session, UserId::from(9), &mut store).await?;
    Ok(())
}

/// The icu delete arm deauthorizes the user, deletes their `icu_users` row,
/// and deletes their domain rows — the admin "delete user" op that used to
/// `todo!()`-panic for icu users.
#[tokio::test]
async fn delete_user_icu_deletes_the_user() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let mut facade = MockEntityFacade::new();
    // The domain delete is the entity-op facade's work (ADR-0012).
    facade.expect_delete_user().returning(|_, _| Ok(()));
    // The provider lookup: no Strava row, an icu row → the icu arm.
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = crate::icu::test_support::icu_user(UserId::from(9), "a1", true);
    let icu_read = icu.clone();
    store
        .icu
        .expect_icuser_get_by_tbid()
        .returning(move |_| Ok(icu_read.clone()));
    store
        .icu
        .expect_icu_events_delete_for_user()
        .returning(|_id| Ok(0));
    let icu_token = icu.clone();
    store
        .icu
        .expect_icuser_update_token()
        .returning(move |_id, _token| Ok(icu_token.clone()));
    store.icu.expect_icuser_delete().returning(|_id| Ok(1));

    let mut session = mock_session_admin(UserId::from(1));
    delete_user(&mut session, UserId::from(9), &facade, &mut store).await?;
    Ok(())
}

// ─── login: both arms ───────────────────────────────────────────────────────

/// The login's Strava arm rejects an insufficient scope (a `BadRequest` before
/// the token exchange).
#[tokio::test]
async fn login_strava_rejects_an_insufficient_scope() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();

    let err = login(Provider::Strava, "code", "read", &facade, &mut store)
        .await
        .expect_err("an insufficient scope is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

/// The login's icu arm rejects an insufficient (present) scope (a
/// `BadRequest` before the token exchange).
#[tokio::test]
async fn login_icu_rejects_an_insufficient_scope() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();

    let err = login(Provider::Icu, "code", "WRONG:SCOPE", &facade, &mut store)
        .await
        .expect_err("an insufficient scope is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

// ─── provider_of: the provider-internal identity lookup ─────────────────────
//
// The single home for "which provider does this user have?": a row in the
// Strava user table means Strava, a row in the icu user table means icu. The
// Strava table is checked first (a user has exactly one provider).

/// A Strava row for the user → the provider is Strava (the strava table is
/// checked first).
#[tokio::test]
async fn provider_of_strava_when_a_strava_row_exists() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let strava = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(move |_| Ok(strava.clone()));

    let provider = crate::provider_of(&mut store, UserId::from(7)).await?;
    assert_eq!(provider, Provider::Strava);
    Ok(())
}

/// No Strava row but an icu row → the provider is icu.
#[tokio::test]
async fn provider_of_icu_when_only_an_icu_row_exists() -> TbResult<()> {
    let mut store = DispatchStore::default();
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = crate::icu::test_support::icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_icuser_get_by_tbid()
        .returning(move |_| Ok(icu.clone()));

    let provider = crate::provider_of(&mut store, UserId::from(9)).await?;
    assert_eq!(provider, Provider::Icu);
    Ok(())
}

/// No row in either table → a `NotFound` (the caller propagates it).
#[tokio::test]
async fn provider_of_not_found_when_no_row_exists() -> TbResult<()> {
    let mut store = DispatchStore::default();
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    store
        .icu
        .expect_icuser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no icu user".into())));

    let err = crate::provider_of(&mut store, UserId::from(7))
        .await
        .expect_err("no row in either table is a NotFound");
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

// ─── sync_users: the provider-agnostic all-users sync ───────────────────────
//
// The admin `trigger_processing` op: queue a sync for every non-disabled user
// on their own provider's lane — a Strava user gets a Strava `Sync` event, an
// icu user is queued via the icu onboarding path.

/// The icu lane: an enabled icu user is queued via the onboarding path.
#[tokio::test]
async fn sync_users_queues_icu_users() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();
    // The icu HTTP seam in test mode: the onboarding's gear + backfill fetch
    // canned empty responses (no gear, no activities → the onboard is a
    // no-op past the queue).
    begin_test();
    queue("/athlete/a1/gear", "[]");
    let today = {
        let dt = time::OffsetDateTime::now_utc();
        format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day())
    };
    queue(
        &format!(
            "/athlete/a1/activities?oldest=1970-01-01&newest={today}&limit=100&fields=id,type,name,start_date,distance,moving_time,elapsed_time,total_elevation_gain,total_elevation_loss,gear,icu_joules"
        ),
        "[]",
    );
    // No Strava row for this user → the `provider_of` lookup resolves icu.
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    // The provider lookup, the lane read, and the onboard's `get_icu_id` all
    // resolve the icu user.
    let icu = crate::icu::test_support::icu_user(UserId::from(9), "a1", true);
    let icu_read = icu.clone();
    store
        .icu
        .expect_icuser_get_by_tbid()
        .returning(move |_| Ok(icu_read.clone()));
    // The backfill's resume cursor: no imported events yet.
    store
        .icu
        .expect_icu_events_get_latest_time()
        .returning(|_id| Ok(None));

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(
        &mut session,
        Some(UserId::from(9)),
        1_700_000_000,
        false,
        &facade,
        &mut store,
    )
    .await?;

    assert_eq!(
        queued,
        vec![UserId::from(9)],
        "the icu user is queued on their own lane"
    );
    Ok(())
}

/// The Strava lane (existing behavior, preserved): an enabled Strava user gets
/// a `Sync` event queued.
#[tokio::test]
async fn sync_users_queues_strava_users() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();
    // The all-users Strava read; no icu users.
    store.strava.expect_stravauser_get_all().returning(|| {
        Ok(vec![crate::strava::test_support::strava_user(
            UserId::from(7),
            42,
            true,
        )])
    });
    store
        .icu
        .expect_icuser_get_all()
        .returning(|| Ok(Vec::new()));
    // The `insert_sync`'s `get_strava_id` read + the queue insert.
    let strava = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(move |_| Ok(strava.clone()));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store
        .strava
        .expect_strava_event_create()
        .returning(move |e| {
            created_rec.lock().unwrap().push(e.clone());
            Ok(())
        });

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(
        &mut session,
        None,
        1_700_000_000,
        false,
        &facade,
        &mut store,
    )
    .await?;

    assert_eq!(queued, vec![UserId::from(7)], "the Strava user is queued");
    let events = created.lock().unwrap().clone();
    assert_eq!(events.len(), 1, "one Sync event is queued");
    assert_eq!(events[0].owner_id, StravaId::from(42));
    Ok(())
}

/// Disabled users (both providers) are skipped (with a log warning) — no sync
/// is queued for them.
#[tokio::test]
async fn sync_users_skips_disabled() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();
    // A disabled Strava user and a disabled icu user.
    store.strava.expect_stravauser_get_all().returning(|| {
        Ok(vec![crate::strava::test_support::strava_user(
            UserId::from(7),
            42,
            false,
        )])
    });
    store.icu.expect_icuser_get_all().returning(|| {
        Ok(vec![crate::icu::test_support::icu_user(
            UserId::from(9),
            "a1",
            false,
        )])
    });

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(
        &mut session,
        None,
        1_700_000_000,
        false,
        &facade,
        &mut store,
    )
    .await?;

    assert!(queued.is_empty(), "disabled users are skipped");
    Ok(())
}

/// A single-user trigger for a Strava user queues a `Sync` event for that
/// user (on their own lane).
#[tokio::test]
async fn sync_users_single_strava() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();
    // The provider lookup, the lane read, and the `insert_sync`'s read all
    // resolve the Strava user.
    let strava = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(move |_| Ok(strava.clone()));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store
        .strava
        .expect_strava_event_create()
        .returning(move |e| {
            created_rec.lock().unwrap().push(e.clone());
            Ok(())
        });

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(
        &mut session,
        Some(UserId::from(7)),
        1_700_000_000,
        false,
        &facade,
        &mut store,
    )
    .await?;

    assert_eq!(
        queued,
        vec![UserId::from(7)],
        "the single Strava user is queued"
    );
    assert_eq!(created.lock().unwrap().len(), 1);
    Ok(())
}

/// A single-user trigger for a user with no row in either provider table is a
/// `NotFound` (the `provider_of` lookup).
#[tokio::test]
async fn sync_users_unknown_user() {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();
    store
        .strava
        .expect_stravauser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    store
        .icu
        .expect_icuser_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no icu user".into())));

    let mut session = mock_session_admin(UserId::from(1));
    let res = sync_users(
        &mut session,
        Some(UserId::from(99)),
        1_700_000_000,
        false,
        &facade,
        &mut store,
    )
    .await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

/// The trigger is admin-only: a non-admin session is `Forbidden` before any
/// store work.
#[tokio::test]
async fn sync_users_forbidden_for_non_admin() {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();
    let mut session = crate::facade::mock_session(UserId::from(1));

    let res = sync_users(
        &mut session,
        None,
        1_700_000_000,
        false,
        &facade,
        &mut store,
    )
    .await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
}
