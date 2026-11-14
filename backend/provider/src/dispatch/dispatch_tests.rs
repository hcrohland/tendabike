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

use tb_domain::{TbResult, UserId};

use crate::Provider;
use crate::icu::test_support::MockIcuDataStore;
use crate::icu::{IcuDataStore, IcuEvent, IcuId, IcuUser};
use crate::strava::event::Event;
use crate::strava::test_support::MockStravaDataStore;
use crate::strava::{StravaDataStore, StravaId, StravaUser};
use crate::test_support::MockEntityFacade;

use super::{accept_event, login};

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

/// The icu webhook secret the intake tests verify against: sets
/// `ICU_WEBHOOK_SECRET` to a known value when it is not already present (a
/// real environment's value takes precedence) and returns the value in force.
fn icu_webhook_secret() -> String {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        if std::env::var("ICU_WEBHOOK_SECRET").is_err() {
            std::env::set_var("ICU_WEBHOOK_SECRET", "icu-test-webhook-secret");
        }
    });
    std::env::var("ICU_WEBHOOK_SECRET").expect("the secret is set by the helper")
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
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
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
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}

// ─── accept_event: the icu arm ──────────────────────────────────────────────

/// The icu arm verifies the secret, filters to `ACTIVITY_ANALYZED`, parses the
/// timestamp, and queues one row per analyzed event — returning the users a
/// row was queued for (the caller's wake list).
#[tokio::test]
async fn accept_event_icu_queues_analyzed_events_only() -> TbResult<()> {
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
                "ACTIVITY_ANALYZED",
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

    // Only the analyzed event was queued (the other type is not subscribed).
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
            "ACTIVITY_ANALYZED",
            "act-1",
            "2024-12-06T06:40:47.011+00:00",
        ),
    );

    let err = accept_event(Provider::Icu, &body, &mut store)
        .await
        .expect_err("a bad secret is rejected");
    assert!(matches!(err, tb_domain::Error::NotAuth(_)));
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
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
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
        &icu_event_json("a1", "ACTIVITY_ANALYZED", "act-1", "not-a-timestamp"),
    );

    let err = accept_event(Provider::Icu, &body, &mut store)
        .await
        .expect_err("an invalid timestamp is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
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
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
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
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}
