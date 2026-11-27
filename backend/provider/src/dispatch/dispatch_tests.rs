//! The dispatch suite (ADR-0014): the `login` and `accept_event` dispatch
//! functions and their match arms, against the store mocks.
//!
//! The dispatch functions are bounded on the single
//! [`Store`](crate::store::Store) interface over each provider's marker
//! (`Store<StravaSpec> + Store<IcuSpec>`), so the suite's store double
//! composes the two providers' [`MockStore`](crate::test_support::MockStore)
//! doubles — delegating field by field. Domain work goes through the
//! entity-op facade (a [`MockEntityFacade`]); the arms' work runs for real
//! against the canned store outcomes. The login's token exchange is the one
//! piece that needs the live provider API, so the login tests cover the
//! arms' scope check (which precedes it).

use std::sync::{Arc, Mutex};

use crate::Provider;
use crate::ProviderStore;
use crate::facade::{
    Error, OnboardingStatus, PartId, PartTypeId, Stat, TbResult, User, UserId, mock_session,
    mock_session_admin,
};
use crate::icu::test_support::{gear_json, icu_user, part};
use crate::icu::{IcuEvent, IcuId, IcuObjectType, IcuSpec, IcuUser};
use crate::store::Store;
use crate::strava::event::Event;
use crate::strava::test_support::strava_user;
use crate::strava::{StravaId, StravaSpec, StravaUser};
use crate::test_support::{
    MockEntityFacade, MockStore, begin_test, icu_webhook_secret, init_test_config_once, queue,
};

use super::{
    accept_event, all_stats, deauthorize, delete_user, gear_url, login, sync_users, user_url,
};

// ─── The composite store double ─────────────────────────────────────────────
//
// The dispatch's `Store<StravaSpec> + Store<IcuSpec>` bound: the two
// providers' store doubles, delegated field by field. The mocks carry the
// canned outcomes the tests assert on.

/// The dispatch suite's store double: [`MockStore<StravaSpec>`] plus
/// [`MockStore<IcuSpec>`], delegated field by field — the
/// `Store<StravaSpec> + Store<IcuSpec>` bound the dispatch functions carry.
/// The single [`MockStore`](crate::test_support::MockStore) double stands in
/// for the per-provider data-store mocks (spec #1029, ticket #1034).
#[derive(Default)]
struct DispatchStore {
    strava: MockStore<StravaSpec>,
    icu: MockStore<IcuSpec>,
}

#[async_trait::async_trait]
impl Store<StravaSpec> for DispatchStore {
    async fn user_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        self.strava.user_get_all().await
    }
    async fn user_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        self.strava.user_get_by_tbid(id).await
    }
    async fn user_get_by_provider_id(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        self.strava.user_get_by_provider_id(id).await
    }
    async fn user_create(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        self.strava.user_create(user).await
    }
    async fn user_update_token(
        &mut self,
        owner: StravaId,
        token: Option<String>,
    ) -> TbResult<StravaUser> {
        self.strava.user_update_token(owner, token).await
    }
    async fn user_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.strava.user_delete(user).await
    }
    async fn user_disable(&mut self, owner: StravaId) -> TbResult<()> {
        self.strava.user_disable(owner).await
    }
    async fn user_event_count(&mut self, owner: &StravaId) -> TbResult<i64> {
        self.strava.user_event_count(owner).await
    }
    async fn event_get_next(&mut self, owner: StravaId) -> TbResult<Option<Event>> {
        self.strava.event_get_next(owner).await
    }
    async fn event_get_later(&mut self, key: i64, owner: StravaId) -> TbResult<Vec<Event>> {
        self.strava.event_get_later(key, owner).await
    }
    async fn event_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.strava.event_delete_batch(values).await
    }
    async fn event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.strava.event_delete(event_id).await
    }
    async fn event_set_time(&mut self, event_id: Option<i32>, time: i64) -> TbResult<()> {
        self.strava.event_set_time(event_id, time).await
    }
    async fn event_delete_sync(&mut self, owner: &StravaId) -> TbResult<usize> {
        self.strava.event_delete_sync(owner).await
    }
    async fn event_create(&mut self, e: Event) -> TbResult<()> {
        self.strava.event_create(e).await
    }
    fn limited_until(&self) -> Option<i64> {
        self.strava.limited_until()
    }
}

#[async_trait::async_trait]
impl Store<IcuSpec> for DispatchStore {
    async fn user_get_all(&mut self) -> TbResult<Vec<IcuUser>> {
        self.icu.user_get_all().await
    }
    async fn user_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser> {
        self.icu.user_get_by_tbid(id).await
    }
    async fn user_get_by_provider_id(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>> {
        self.icu.user_get_by_provider_id(id).await
    }
    async fn user_create(&mut self, user: IcuUser) -> TbResult<IcuUser> {
        self.icu.user_create(user).await
    }
    async fn user_update_token(
        &mut self,
        owner: IcuId,
        token: Option<String>,
    ) -> TbResult<IcuUser> {
        self.icu.user_update_token(owner, token).await
    }
    async fn user_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.icu.user_delete(user).await
    }
    async fn user_disable(&mut self, owner: IcuId) -> TbResult<()> {
        self.icu.user_disable(owner).await
    }
    async fn user_event_count(&mut self, owner: &IcuId) -> TbResult<i64> {
        self.icu.user_event_count(owner).await
    }
    async fn event_get_next(&mut self, owner: IcuId) -> TbResult<Option<IcuEvent>> {
        self.icu.event_get_next(owner).await
    }
    async fn event_get_later(&mut self, key: String, owner: IcuId) -> TbResult<Vec<IcuEvent>> {
        self.icu.event_get_later(key, owner).await
    }
    async fn event_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.icu.event_delete_batch(values).await
    }
    async fn event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.icu.event_delete(event_id).await
    }
    async fn event_set_time(&mut self, event_id: Option<i32>, time: i64) -> TbResult<()> {
        self.icu.event_set_time(event_id, time).await
    }
    async fn event_delete_sync(&mut self, owner: &IcuId) -> TbResult<usize> {
        self.icu.event_delete_sync(owner).await
    }
    async fn event_create(&mut self, e: IcuEvent) -> TbResult<()> {
        self.icu.event_create(e).await
    }
    fn limited_until(&self) -> Option<i64> {
        self.icu.limited_until()
    }
}

/// The composite double satisfies the [`ProviderStore`] supertrait (the
/// "all provider stores" bound the dispatch fns and `provider_of` carry) with
/// an empty body — the per-provider [`MockStore`] halves already carry the
/// `Store` markers and the `Send + Sync` liveness.
impl ProviderStore for DispatchStore {}

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

// ─── accept_event: the Strava arm ───────────────────────────────────────────

/// The Strava arm parses the raw body and queues the event (the `strava_events`
/// row), returning the owner's TendaBike id (the caller's wake).
#[tokio::test]
async fn accept_event_strava_queues_an_activity_event() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    let user = strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_user_get_by_provider_id()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(None));
    let created_cell = created.clone();
    store.strava.expect_event_create().returning(move |e| {
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
    init_test_config_once();
    let mut store = DispatchStore::default();
    let user = strava_user(UserId::from(7), 42, true);
    let lookup = user.clone();
    store
        .strava
        .expect_user_get_by_provider_id()
        .returning(move |_id| Ok(Some(lookup.clone())));
    store
        .strava
        .expect_user_disable()
        .returning(|_owner| Ok(()));

    let queued = accept_event(Provider::Strava, &strava_deauth_body(42), &mut store).await?;

    assert!(queued.is_empty(), "a deauth queues nothing (no wake)");
    Ok(())
}

/// The Strava arm rejects an unknown owner (the `strava_events` key would have
/// no user) — a `BadRequest`, nothing queued.
#[tokio::test]
async fn accept_event_strava_unknown_owner_is_rejected() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    store
        .strava
        .expect_user_get_by_provider_id()
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
    init_test_config_once();
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
    let user = icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_user_get_by_provider_id()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_cell = created.clone();
    store.icu.expect_event_create().returning(move |e| {
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
    init_test_config_once();
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
    init_test_config_once();
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

/// The icu arm queues an **`ACTIVITY_DELETED`** event — it is a
/// subscribed type, and the queued row carries the event type (the queue's
/// delete dispatch key).
#[tokio::test]
async fn accept_event_icu_queues_a_deleted_event() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let user = icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_user_get_by_provider_id()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_cell = created.clone();
    store.icu.expect_event_create().returning(move |e| {
        created_cell
            .lock()
            .expect("the capture is not poisoned")
            .push(e.clone());
        Ok(())
    });

    let body = icu_body(
        &secret,
        &icu_event_json(
            "a1",
            "ACTIVITY_DELETED",
            "act-1",
            "2024-12-06T06:40:47.011+00:00",
        ),
    );
    let queued = accept_event(Provider::Icu, &body, &mut store).await?;

    // The deleted event is subscribed and queued (the wake list).
    assert_eq!(queued, vec![UserId::from(9)]);
    let created = created.lock().expect("the capture is not poisoned");
    assert_eq!(created.len(), 1);
    let event = &created[0];
    assert_eq!(event.icu_athlete_id, IcuId::from("a1"));
    assert_eq!(event.activity_id, "act-1");
    assert_eq!(event.event_type, "ACTIVITY_DELETED");
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
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = icu_user(UserId::from(9), "a1", true);
    let icu_read = icu.clone();
    store
        .icu
        .expect_user_get_by_tbid()
        .returning(move |_| Ok(icu_read.clone()));
    store.icu.expect_user_disable().returning(|_owner| Ok(()));

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
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = icu_user(UserId::from(9), "a1", true);
    let icu_read = icu.clone();
    store
        .icu
        .expect_user_get_by_tbid()
        .returning(move |_| Ok(icu_read.clone()));
    store.icu.expect_user_disable().returning(|_owner| Ok(()));
    store.icu.expect_user_delete().returning(|_id| Ok(1));

    let mut session = mock_session_admin(UserId::from(1));
    delete_user(&mut session, UserId::from(9), &facade, &mut store).await?;
    Ok(())
}

// ─── login: both arms ───────────────────────────────────────────────────────

/// The login's Strava arm rejects an insufficient scope (a `BadRequest` before
/// the token exchange). The Strava flow is a plain authorization-code
/// exchange: the (PKCE) code verifier is not used by it.
#[tokio::test]
async fn login_strava_rejects_an_insufficient_scope() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();

    let err = login(Provider::Strava, "code", "read", None, &facade, &mut store)
        .await
        .expect_err("an insufficient scope is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

/// The login's icu arm rejects an insufficient (present) scope (a
/// `BadRequest` before the token exchange).
#[tokio::test]
async fn login_icu_rejects_an_insufficient_scope() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();

    let err = login(
        Provider::Icu,
        "code",
        "WRONG:SCOPE",
        Some("verifier"),
        &facade,
        &mut store,
    )
    .await
    .expect_err("an insufficient scope is rejected");
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

/// The login's icu arm rejects a missing code verifier (a `BadRequest`
/// before the token exchange): the web layer always recovers the verifier
/// from the signed state, so its absence is a protocol violation.
#[tokio::test]
async fn login_icu_rejects_a_missing_code_verifier() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let facade = MockEntityFacade::new();

    let err = login(Provider::Icu, "code", "", None, &facade, &mut store)
        .await
        .expect_err("a missing verifier is rejected");
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
    let strava = strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_user_get_by_tbid()
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
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_user_get_by_tbid()
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
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    store
        .icu
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no icu user".into())));

    let err = crate::provider_of(&mut store, UserId::from(7))
        .await
        .expect_err("no row in either table is a NotFound");
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

// ─── gear_url: the provider-neutral gear redirect URL ──────────────────────
//
// The op derives the part's real provider in-transaction from the part's
// owner (the `provider_of` lookup — the same identity rule as
// `deauthorize` / `delete_user`), not on the route: the part read is the
// ownership check, and the provider arm is picked from the derived provider.

/// A part owned by a Strava user takes the Strava arm — the part's source
/// builds the Strava gear URL.
#[tokio::test]
async fn gear_url_strava_owner_builds_the_strava_url() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let mut facade = MockEntityFacade::new();
    // The part read (the ownership check) returns the sourced part; the
    // owner's provider lookup finds the Strava row.
    let part = part(
        PartId::from(7),
        UserId::from(7),
        PartTypeId::from(1),
        Some("b42".to_string()),
    );
    let part_read = part.clone();
    facade
        .expect_part()
        .times(1)
        .returning(move |_id, _user, _store| Ok(part_read.clone()));
    let strava = strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_user_get_by_tbid()
        .times(1)
        .returning(move |_| Ok(strava.clone()));

    let url = gear_url(
        Box::new(mock_session(UserId::from(7))),
        PartId::from(7),
        &facade,
        &mut store,
    )
    .await?;
    assert_eq!(url, Some("https://strava.com/bikes/42".to_string()));
    Ok(())
}

/// A part owned by an icu user takes the icu arm — the typed absence
/// (`None`): icu gear is a private settings item with no public page.
#[tokio::test]
async fn gear_url_icu_owner_is_the_typed_absence() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let mut facade = MockEntityFacade::new();
    let part = part(
        PartId::from(7),
        UserId::from(9),
        PartTypeId::from(1),
        Some("icu:g1".to_string()),
    );
    let part_read = part.clone();
    facade
        .expect_part()
        .times(1)
        .returning(move |_id, _user, _store| Ok(part_read.clone()));
    // The owner's provider lookup: no Strava row, an icu row → icu.
    store
        .strava
        .expect_user_get_by_tbid()
        .times(1)
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_user_get_by_tbid()
        .times(1)
        .returning(move |_| Ok(icu.clone()));

    let url = gear_url(
        Box::new(mock_session(UserId::from(9))),
        PartId::from(7),
        &facade,
        &mut store,
    )
    .await?;
    assert_eq!(url, None);
    Ok(())
}

/// A failed part read (the ownership check's `Forbidden`) propagates — the
/// route maps a failed op to home (`/`).
#[tokio::test]
async fn gear_url_propagates_a_failed_part_read() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let mut facade = MockEntityFacade::new();
    facade
        .expect_part()
        .times(1)
        .returning(|_id, _user, _store| Err(Error::Forbidden("not your part".to_string())));

    let res = gear_url(
        Box::new(mock_session(UserId::from(2))),
        PartId::from(7),
        &facade,
        &mut store,
    )
    .await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}

// ─── user_url: the provider-neutral user redirect URL ──────────────────────
//
// The op derives the target's real provider in-transaction from the target's
// tbid (the `provider_of` lookup — the same identity rule as
// `deauthorize` / `delete_user`), not on the route. A pure data-store read:
// no facade.

/// A Strava target takes the Strava arm — the public athlete profile URL.
#[tokio::test]
async fn user_url_strava_target_builds_the_strava_url() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let strava = strava_user(UserId::from(3), 42, true);
    // The user row is read twice: the `provider_of` lookup, then the driver's
    // `strava_url` (the athlete id for the public profile URL).
    store
        .strava
        .expect_user_get_by_tbid()
        .times(2)
        .returning(move |_| Ok(strava.clone()));

    let url = user_url(UserId::from(3), &mut store).await?;
    assert_eq!(url, "https://strava.com/athletes/42");
    Ok(())
}

/// An icu target takes the icu arm — the public athlete profile URL built
/// from the athlete id in the icu user row.
#[tokio::test]
async fn user_url_icu_target_builds_the_icu_url() -> TbResult<()> {
    let mut store = DispatchStore::default();
    // The target's provider lookup: no Strava row, an icu row → icu.
    store
        .strava
        .expect_user_get_by_tbid()
        .times(1)
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    let icu = icu_user(UserId::from(3), "a1", true);
    // The icu row is read twice: the `provider_of` lookup, then the driver's
    // `user_url` (the athlete id for the public profile URL).
    store
        .icu
        .expect_user_get_by_tbid()
        .times(2)
        .returning(move |_| Ok(icu.clone()));

    let url = user_url(UserId::from(3), &mut store).await?;
    assert_eq!(url, "https://intervals.icu/athletes/a1");
    Ok(())
}

// ─── sync_users: the provider-agnostic all-users sync ───────────────────────
//
// The admin `trigger_processing` op: queue a sync for every non-disabled user
// on their own provider's lane — a Strava user gets a Strava `Sync` event, an
// icu user is queued via the icu onboarding path.

/// The icu lane: an enabled icu user gets a `SYNC` event queued — a pure
/// activity re-sync, no gear import (gear import is an onboarding-only
/// concern).
#[tokio::test]
async fn sync_users_queues_icu_users() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    let mut facade: MockEntityFacade<DispatchStore> = MockEntityFacade::new();
    // The icu HTTP seam in test mode: the gear list is non-empty, so a gear
    // import WOULD call the gear store — the re-trigger must not.
    begin_test();
    queue(
        "/athlete/a1/gear",
        &format!("[{}]", gear_json("g1", "Bike")),
    );
    // The re-trigger is a pure activity re-sync: the gear store is not called.
    facade.expect_part_by_source().times(0);
    facade.expect_create_part().times(0);
    // No Strava row for this user → the `provider_of` lookup resolves icu.
    store
        .strava
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    // The provider lookup, the lane read, and the `insert_sync`'s
    // `get_icu_id` all resolve the icu user.
    let icu = icu_user(UserId::from(9), "a1", true);
    let icu_read = icu.clone();
    store
        .icu
        .expect_user_get_by_tbid()
        .returning(move |_| Ok(icu_read.clone()));
    // The `insert_sync`'s REPLACE: no existing `SYNC` row to drop, then the
    // fresh `SYNC` row is queued.
    store.icu.expect_event_delete_sync().returning(|_id| Ok(0));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store.icu.expect_event_create().returning(move |e| {
        created_rec.lock().unwrap().push(e);
        Ok(())
    });

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(
        &mut session,
        Some(UserId::from(9)),
        1_700_000_000,
        &mut store,
    )
    .await?;

    assert_eq!(
        queued,
        vec![UserId::from(9)],
        "the icu user is queued on their own lane"
    );
    let events = created.lock().unwrap().clone();
    assert_eq!(events.len(), 1, "one SYNC event is queued");
    assert_eq!(
        events[0].object_type,
        IcuObjectType::Sync,
        "the queued row is a SYNC"
    );
    assert_eq!(
        events[0].oldest,
        Some(1_700_000_000),
        "the SYNC's lower bound is the trigger time"
    );
    assert_eq!(events[0].event_time, 0, "the newest sentinel is 0");
    Ok(())
}

/// The Strava lane (existing behavior, preserved): an enabled Strava user gets
/// a `Sync` event queued.
#[tokio::test]
async fn sync_users_queues_strava_users() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    // The all-users Strava read; no icu users.
    store
        .strava
        .expect_user_get_all()
        .returning(|| Ok(vec![strava_user(UserId::from(7), 42, true)]));
    store.icu.expect_user_get_all().returning(|| Ok(Vec::new()));
    // The `insert_sync`'s `get_owner_id` read + the queue insert.
    let strava = strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_user_get_by_tbid()
        .returning(move |_| Ok(strava.clone()));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store.strava.expect_event_create().returning(move |e| {
        created_rec.lock().unwrap().push(e.clone());
        Ok(())
    });

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(&mut session, None, 1_700_000_000, &mut store).await?;

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
    init_test_config_once();
    let mut store = DispatchStore::default();
    // A disabled Strava user and a disabled icu user.
    store
        .strava
        .expect_user_get_all()
        .returning(|| Ok(vec![strava_user(UserId::from(7), 42, false)]));
    store
        .icu
        .expect_user_get_all()
        .returning(|| Ok(vec![icu_user(UserId::from(9), "a1", false)]));

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(&mut session, None, 1_700_000_000, &mut store).await?;

    assert!(queued.is_empty(), "disabled users are skipped");
    Ok(())
}

/// A single-user trigger for a Strava user queues a `Sync` event for that
/// user (on their own lane).
#[tokio::test]
async fn sync_users_single_strava() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    // The provider lookup, the lane read, and the `insert_sync`'s read all
    // resolve the Strava user.
    let strava = strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_user_get_by_tbid()
        .returning(move |_| Ok(strava.clone()));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store.strava.expect_event_create().returning(move |e| {
        created_rec.lock().unwrap().push(e.clone());
        Ok(())
    });

    let mut session = mock_session_admin(UserId::from(1));
    let queued = sync_users(
        &mut session,
        Some(UserId::from(7)),
        1_700_000_000,
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
    store
        .strava
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no strava user".into())));
    store
        .icu
        .expect_user_get_by_tbid()
        .returning(|_| Err(Error::NotFound("no icu user".into())));

    let mut session = mock_session_admin(UserId::from(1));
    let res = sync_users(
        &mut session,
        Some(UserId::from(99)),
        1_700_000_000,
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
    let mut session = crate::facade::mock_session(UserId::from(1));

    let res = sync_users(&mut session, None, 1_700_000_000, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
}

// ─── all_stats: the provider-agnostic all-user stat read ────────────────────
//
// The admin `GET /user/all` read: every enabled provider's users, each row
// stamped with its provider's wire value by the driver's `all_stats` — the
// admin's list is provider-agnostic (an admin of one provider sees the
// other provider's users too).

/// Both lanes: a Strava user and an icu user each appear in the list,
/// stamped with their provider's wire value, in the `Provider` enum's
/// definition order (Strava first). The admin's provider is irrelevant —
/// the read is not the admin's own lane.
#[tokio::test]
async fn all_stats_lists_users_of_both_providers() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    let mut facade: MockEntityFacade<DispatchStore> = MockEntityFacade::new();
    // The all-user reads: one Strava user, one icu user.
    store
        .strava
        .expect_user_get_all()
        .returning(|| Ok(vec![strava_user(UserId::from(7), 42, true)]));
    store
        .icu
        .expect_user_get_all()
        .returning(|| Ok(vec![icu_user(UserId::from(9), "a1", true)]));
    // The per-user queued-event counts (the driver's `all_stats` read).
    store
        .strava
        .expect_user_event_count()
        .returning(|_owner| Ok(3));
    store
        .icu
        .expect_user_event_count()
        .returning(|_owner| Ok(1));
    // The domain stat per user.
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

    let stats = all_stats(&facade, &mut store).await?;

    assert_eq!(stats.len(), 2, "both providers' users appear");
    assert_eq!(
        stats[0].stat.user.id,
        UserId::from(7),
        "the Strava user is first (the enum order)"
    );
    assert_eq!(
        stats[0].provider, "strava",
        "the Strava row is stamped strava"
    );
    assert_eq!(stats[0].events, 3);
    assert!(!stats[0].disabled);
    assert_eq!(
        stats[1].stat.user.id,
        UserId::from(9),
        "the icu user appears"
    );
    assert_eq!(stats[1].provider, "icu", "the icu row is stamped icu");
    assert_eq!(stats[1].events, 1);
    assert!(!stats[1].disabled);
    Ok(())
}

/// A disabled user's row still appears (with its `disabled` flag set) — the
/// admin list shows disabled users, it does not skip them (the skip is the
/// sync trigger's, not the stat read's).
#[tokio::test]
async fn all_stats_includes_disabled_users_flagged() -> TbResult<()> {
    init_test_config_once();
    let mut store = DispatchStore::default();
    let mut facade: MockEntityFacade<DispatchStore> = MockEntityFacade::new();
    // A disabled Strava user; no icu users.
    store
        .strava
        .expect_user_get_all()
        .returning(|| Ok(vec![strava_user(UserId::from(7), 42, false)]));
    store.icu.expect_user_get_all().returning(|| Ok(Vec::new()));
    store
        .strava
        .expect_user_event_count()
        .returning(|_owner| Ok(0));
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

    let stats = all_stats(&facade, &mut store).await?;

    assert_eq!(stats.len(), 1, "the disabled user appears");
    assert!(stats[0].disabled, "the disabled flag is set");
    Ok(())
}
