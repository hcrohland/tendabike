use super::super::test_support::{activity_json, icu_user, setup, stub_activity_json};
use super::*;
use crate::facade::{
    ActivityId, Notification, NotificationKind, NotificationLifecycle, NotificationState,
    NotificationSubject, Summary,
};
use crate::test_support::icu_webhook_secret;
use crate::test_support::{queue, queue_error};
use time::OffsetDateTime;

fn event(id: i32, activity: &str) -> IcuEvent {
    IcuEvent {
        id: Some(id),
        icu_athlete_id: "a1".into(),
        activity_id: activity.to_string(),
        event_time: 1000,
        object_type: IcuObjectType::Activity,
        event_type: "ACTIVITY_UPLOADED".to_string(),
        oldest: None,
    }
}

/// An `ACTIVITY_DELETED` event: the queue's delete dispatch key.
fn deleted_event(id: i32, activity: &str) -> IcuEvent {
    let mut e = event(id, activity);
    e.event_type = "ACTIVITY_DELETED".into();
    e
}

fn notification() -> Notification {
    Notification {
        id: uuid::Uuid::now_v7().into(),
        recipient: UserId::from(1),
        lifecycle: NotificationLifecycle::Transient,
        kind: NotificationKind::StubsSkipped,
        subject: NotificationSubject::StubsSkipped,
        message: Some("1".to_string()),
        state: NotificationState::New,
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
}

// ─── accept ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn accept_queues_event_and_returns_tbid() {
    let (mut store, _facade) = setup();
    let e = event(1, "10");
    store
        .expect_icuser_get_by_icuid()
        .returning(|_id| Ok(Some(icu_user(UserId::from(1), "a1", true))));
    store.expect_icu_event_create().returning(|_e| Ok(()));
    let tbid = e.accept(&mut store).await.unwrap();
    assert_eq!(tbid, UserId::from(1));
}

#[tokio::test]
async fn accept_unknown_athlete_is_bad_request() {
    let (mut store, _facade) = setup();
    let e = event(1, "10");
    store.expect_icuser_get_by_icuid().returning(|_id| Ok(None));
    let res = e.accept(&mut store).await;
    assert!(matches!(res, Err(Error::BadRequest(_))));
}

// ─── process (one event per call) ───────────────────────────────────────────

/// Canned the `get_event` read of a single queued event: the event is both the
/// "next" event and the only "later" event for its activity (the dedup no-ops).
fn queue_event(store: &mut MockIcuDataStore, event: IcuEvent) {
    let next = event.clone();
    let later = event.clone();
    store
        .expect_icu_event_get_next_for_user()
        .returning(move |_id| Ok(Some(next.clone())));
    store
        .expect_icu_event_get_later()
        .returning(move |_activity, _icu| Ok(vec![later.clone()]));
}

#[tokio::test]
async fn process_imports_activity() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    // One event: the kit's `get_event` reads it (the dedup no-ops).
    queue_event(&mut store, event(1, "10"));
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    queue("/activity/10", &activity_json("10", "Ride", None));
    facade
        .expect_upsert_activity()
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.notifications.is_empty());
}

/// A stub activity (both `distance` and `moving_time` null) is skipped with
/// one transient notice per event (spec #764; spec #1023 makes it per-event).
#[tokio::test]
async fn process_stub_emits_one_notice() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    queue_event(&mut store, event(1, "10"));
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    queue("/activity/10", &stub_activity_json("10", "Ride"));
    // The one per-event notice: the facade's create_notification returns it.
    facade
        .expect_create_notification()
        .returning(|_r, _l, _k, _s, message, _store| {
            assert_eq!(
                message.as_deref(),
                Some("1"),
                "the skip count is 1 (per event)"
            );
            Ok(notification())
        });
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    assert!(summary.activities.is_empty());
    assert_eq!(summary.notifications.len(), 1);
}

#[tokio::test]
async fn process_401_disables_user() {
    let (mut store, facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    queue_event(&mut store, event(1, "10"));
    // The 401 disables the user: the events are dropped and the token cleared.
    store
        .expect_icu_events_delete_for_user()
        .returning(|_id| Ok(1));
    store
        .expect_icuser_update_token()
        .returning(|_id, _t| Ok(icu_user(UserId::from(1), "a1", false)));
    queue_error("/activity/10", Error::NotAuth("revoked".to_string()));
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    assert!(summary.is_empty());
}

#[tokio::test]
async fn process_429_keeps_event_queued() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    queue_event(&mut store, event(1, "10"));
    // The 429 keeps the event queued: it is NOT deleted (it stays queued).
    store.expect_icu_event_delete().times(0);
    queue_error("/activity/10", Error::TryAgain("rate limited"));
    let summary = process(UserId::from(1), &_facade, &mut store)
        .await
        .unwrap();
    assert!(summary.is_empty());
}

/// A permanently bad event (a 404) is dropped from the queue — the delete is
/// the kit's `fail_event`'s, in exactly one place (no second delete on the
/// caller's fall-through, spec #1023).
#[tokio::test]
async fn process_permanent_error_drops_event() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    queue_event(&mut store, event(1, "10"));
    // The poison event is deleted (in exactly one place).
    let deleted: std::sync::Arc<std::sync::Mutex<Option<Option<i32>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let rec = deleted.clone();
    store.expect_icu_event_delete().returning(move |id| {
        rec.lock().unwrap().replace(id);
        Ok(())
    });
    queue_error("/activity/10", Error::NotFound("gone".to_string()));
    let summary = process(UserId::from(1), &_facade, &mut store)
        .await
        .unwrap();
    assert!(summary.is_empty());
    assert_eq!(
        deleted.lock().unwrap().as_ref(),
        Some(&Some(1)),
        "the poison event is deleted"
    );
}

// ─── process: ACTIVITY_DELETED ──────────────────────────────────────────────

#[tokio::test]
async fn process_deletes_activity_on_deleted_event() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    // One ACTIVITY_DELETED event: the kit's `get_event` reads it.
    queue_event(&mut store, deleted_event(1, "10"));
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    // The delete returns the domain's summary (the activity tombstone).
    facade
        .expect_delete_activity()
        .returning(|id, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.tombstone(id);
            Ok(summary)
        });
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    // The deleted activity is reported as a tombstone (the client's merge drops it).
    assert_eq!(summary.activities.len(), 1);
    assert!(
        summary
            .activities
            .is_tombstoned(&ActivityId::from("icu:10"))
    );
}

/// `process` on a resolved `SYNC` row runs its walk (not the import path,
/// which would 404 and drop the durable cursor row).
#[tokio::test]
async fn process_routes_sync_row_to_its_walk() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    let now = now_ts();
    // A resolved `SYNC` row (event_time = now): the dedup no-ops (the `"sync"`
    // sentinel key groups with nothing else).
    queue_event(&mut store, sync_event(1, 0, now));
    // The walk's completion deletes the row.
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    // The `SYNC` walk's batch (< 100 → end of history → the row is deleted).
    queue(
        &sync_uri(0, now),
        &format!("[{}]", activity_json("30", "Ride", None)),
    );
    facade
        .expect_upsert_activity()
        .returning(|act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });

    let summary = super::process(UserId::from(1), &facade, &mut store)
        .await
        .unwrap();

    assert_eq!(
        summary.activities.len(),
        1,
        "the walk's activity rides the returned Summary"
    );
    assert!(
        summary
            .activities
            .get(&ActivityId::from("icu:30"))
            .is_some()
    );
    // The walk fetched the list URI (not an import of the `"sync"` sentinel).
    let reqs = crate::test_support::requests();
    assert!(
        reqs.iter().any(|r| r.starts_with("/athlete/a1/activities")),
        "the SYNC row went through its walk's list fetch"
    );
}

// ─── insert_sync: the durable backfill cursor ───────────────────────────────

/// A queued `SYNC` event: the `oldest` bound and the `newest` cursor
/// (`event_time`).
fn sync_event(id: i32, oldest: i64, newest: i64) -> IcuEvent {
    IcuEvent {
        id: Some(id),
        icu_athlete_id: "a1".into(),
        activity_id: "sync".to_string(),
        event_time: newest,
        object_type: IcuObjectType::Sync,
        event_type: String::new(),
        oldest: Some(oldest),
    }
}

/// The `SYNC` walk's list-fetch URI for the test athlete (`a1`).
fn sync_uri(oldest: i64, newest: i64) -> String {
    format!(
        "/athlete/a1/activities?oldest={}&newest={}&limit=100&fields={}",
        super::date_str(oldest),
        super::date_str(newest),
        super::FIELDS
    )
}

/// The store fixture for the `SYNC` tests: the user lookup resolves the
/// athlete id (the `insert_sync`'s `get_icu_id`).
fn sync_user_fixture(store: &mut MockIcuDataStore) {
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
}

type MockIcuDataStore = super::super::traits::MockIcuDataStore;

/// `insert_sync` enqueues a `SYNC` event with the `oldest` bound and the `0`
/// newest sentinel.
#[tokio::test]
async fn insert_sync_enqueues_sync_event() {
    let (mut store, _facade) = setup();
    sync_user_fixture(&mut store);
    store
        .expect_icu_events_delete_sync_for_user()
        .returning(|_id| Ok(0));
    let created: std::sync::Arc<std::sync::Mutex<Vec<IcuEvent>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = created.clone();
    store.expect_icu_event_create().returning(move |e| {
        rec.lock().unwrap().push(e);
        Ok(())
    });

    super::insert_sync(UserId::from(1), 1_700_000_000, &mut store)
        .await
        .unwrap();

    let events = created.lock().unwrap().clone();
    assert_eq!(events.len(), 1, "one SYNC event is queued");
    assert_eq!(events[0].object_type, IcuObjectType::Sync);
    assert_eq!(
        events[0].oldest,
        Some(1_700_000_000),
        "the lower bound is the trigger time"
    );
    assert_eq!(events[0].event_time, 0, "the newest sentinel is 0");
    assert_eq!(events[0].activity_id, "sync", "the activity_id sentinel");
}

/// `insert_sync` is REPLACE: it drops the user's existing `SYNC` row before
/// queueing the fresh one (at most one `SYNC` per user).
#[tokio::test]
async fn insert_sync_replaces_existing_sync() {
    let (mut store, _facade) = setup();
    sync_user_fixture(&mut store);
    // The REPLACE: the existing `SYNC` row is deleted (1 row) before the fresh
    // one is queued.
    let deletes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let del = deletes.clone();
    store
        .expect_icu_events_delete_sync_for_user()
        .returning(move |_id| {
            del.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(1)
        });
    store.expect_icu_event_create().returning(|_e| Ok(()));

    // Two enqueues: each replaces the prior `SYNC` row.
    super::insert_sync(UserId::from(1), 0, &mut store)
        .await
        .unwrap();
    super::insert_sync(UserId::from(1), 1_700_000_000, &mut store)
        .await
        .unwrap();

    assert_eq!(
        deletes.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "each enqueue drops the prior SYNC row"
    );
}

// ─── process_sync: the durable backfill cursor walk ─────────────────────────

/// The test's `now` (the zero-sentinel's resolution and the first query's
/// `newest` bound).
fn now_ts() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// `process` on a `SYNC` event resolves the `0` sentinel to now, fetches one
/// batch over `[oldest, newest]`, and on a `< 100` batch deletes the row and
/// returns the batch's `Summary`.
#[tokio::test]
async fn process_sync_short_batch_deletes_row() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    // The queue holds one `SYNC` row (the `0` sentinel, full history).
    queue_event(&mut store, sync_event(1, 0, 0));
    // The zero-sentinel resolution persists `now`; the completion deletes the
    // row.
    store
        .expect_icu_event_set_time()
        .returning(|_id, _t| Ok(()));
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    // One batch of activities (< 100 → end of history).
    queue(
        &sync_uri(0, now_ts()),
        &format!(
            "[{},{}]",
            activity_json("10", "Ride", None),
            activity_json("11", "Ride", None)
        ),
    );
    facade
        .expect_upsert_activity()
        .times(2)
        .returning(|act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });

    let summary = super::process(UserId::from(1), &facade, &mut store)
        .await
        .unwrap();

    assert_eq!(
        summary.activities.len(),
        2,
        "the batch's imported activities ride the returned Summary"
    );
}

/// `process` on a `SYNC` event whose fetch 429s: the cursor is *not* advanced
/// past the 429 and the row is *not* deleted (a later `process` resumes from
/// the stored cursor).
#[tokio::test]
async fn process_sync_429_keeps_cursor_and_row() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    queue_event(&mut store, sync_event(1, 0, 0));
    // The zero-sentinel resolution persists `now` (the cursor is set before
    // the fetch). No `icu_event_delete` is expected: the 429 keeps the row
    // queued.
    store
        .expect_icu_event_set_time()
        .returning(|_id, _t| Ok(()));
    queue_error(&sync_uri(0, now_ts()), Error::TryAgain("rate limited"));
    // No activities are imported (the 429 precedes any import).
    facade.expect_upsert_activity().times(0);

    let summary = super::process(UserId::from(1), &facade, &mut store)
        .await
        .unwrap();

    assert!(
        summary.is_empty(),
        "a 429 imports nothing and leaves the row queued"
    );
}

/// `process` on a `SYNC` event with a full (100) batch: the persisted `newest`
/// cursor advances to the batch's oldest activity and the row stays queued for
/// the next `process` (one batch per call — no second fetch).
#[tokio::test]
async fn process_sync_full_batch_advances_cursor_and_keeps_row() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    // The queue holds one `SYNC` row (the `0` sentinel, full history).
    queue_event(&mut store, sync_event(1, 0, 0));
    // The zero-sentinel resolution + the batch's cursor advance are
    // persisted. No `icu_event_delete` is expected: a full batch leaves the
    // row queued.
    let set_times: std::sync::Arc<std::sync::Mutex<Vec<i64>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = set_times.clone();
    store.expect_icu_event_set_time().returning(move |_id, t| {
        rec.lock().unwrap().push(t);
        Ok(())
    });
    store.expect_icu_event_delete().times(0);
    // A full batch (100) → the cursor advances to the oldest activity.
    let acts: Vec<String> = (0..100)
        .map(|i| activity_json(&format!("100{}", i), "Ride", None))
        .collect();
    queue(&sync_uri(0, now_ts()), &format!("[{}]", acts.join(",")));
    // The oldest activity in the batch (all share `activity_json`'s start
    // date, 2026-01-02T10:00:00Z).
    use time::format_description::well_known::Rfc3339;
    let oldest_ts = OffsetDateTime::parse("2026-01-02T10:00:00Z", &Rfc3339)
        .unwrap()
        .unix_timestamp();

    facade
        .expect_upsert_activity()
        .times(100)
        .returning(|act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });

    let summary = super::process(UserId::from(1), &facade, &mut store)
        .await
        .unwrap();

    assert_eq!(
        summary.activities.len(),
        100,
        "the batch's activities are imported"
    );
    let times = set_times.lock().unwrap().clone();
    // The first `set_time` is the zero-sentinel resolution (now); the second
    // is the cursor advance to the oldest activity in the full batch.
    assert_eq!(
        times.len(),
        2,
        "the sentinel resolution and the one cursor advance"
    );
    assert_eq!(
        times[1], oldest_ts,
        "the persisted cursor equals the batch's oldest activity"
    );
}

/// `process` on a `SYNC` event with a non-zero `newest` (a stored cursor from
/// a prior, interrupted walk) resumes from that cursor — it does not re-resolve
/// the sentinel.
#[tokio::test]
async fn process_sync_resumes_from_stored_cursor() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    // A `SYNC` row whose `newest` cursor is a stored (non-zero) time: the
    // resume point of a prior, interrupted walk.
    let cursor = now_ts() - 86400; // one day before now
    queue_event(&mut store, sync_event(1, 0, cursor));
    // The stored cursor is non-zero: the sentinel is NOT re-resolved, so no
    // `icu_event_set_time` at all (the short batch completes and deletes the
    // row).
    store.expect_icu_event_set_time().times(0);
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    // The batch is bounded by the stored cursor (not now).
    queue(
        &sync_uri(0, cursor),
        &format!("[{}]", activity_json("20", "Ride", None)),
    );
    facade
        .expect_upsert_activity()
        .times(1)
        .returning(|act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });

    let summary = super::process(UserId::from(1), &facade, &mut store)
        .await
        .unwrap();

    assert_eq!(
        summary.activities.len(),
        1,
        "the batch resumes from the stored cursor"
    );
    // The fetch used the stored cursor (one day before now), not now.
    let reqs = crate::test_support::requests();
    assert_eq!(reqs.len(), 1, "exactly one batch is fetched");
    assert!(
        reqs[0].contains(&format!("newest={}", super::date_str(cursor))),
        "the fetch is bounded by the stored cursor"
    );
}

/// `process` on a `SYNC` event where a full (100) batch does not clear the
/// start day (the batch's oldest activity is still on it): the persisted
/// cursor steps to the start day minus one (re-querying the same day would
/// loop) and the row stays queued.
#[tokio::test]
async fn process_sync_full_batch_on_start_day_steps_back_a_day() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    use time::format_description::well_known::Rfc3339;
    // A `SYNC` row whose stored cursor (the start day) is the activities' day:
    // a full batch cannot clear it.
    let cursor = OffsetDateTime::parse("2026-01-02T23:00:00Z", &Rfc3339)
        .unwrap()
        .unix_timestamp();
    queue_event(&mut store, sync_event(1, 0, cursor));
    // The cursor is non-zero (no sentinel resolution); the one `set_time` is
    // the cursor step back to the day before the start day.
    let set_times: std::sync::Arc<std::sync::Mutex<Vec<i64>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = set_times.clone();
    store.expect_icu_event_set_time().returning(move |_id, t| {
        rec.lock().unwrap().push(t);
        Ok(())
    });
    // The row stays queued (the next `process` resumes from the stepped-back
    // cursor).
    store.expect_icu_event_delete().times(0);

    // A full batch (100), all on the start day (2026-01-02).
    let acts: Vec<String> = (0..100)
        .map(|i| activity_json(&format!("100{}", i), "Ride", None))
        .collect();
    queue(&sync_uri(0, cursor), &format!("[{}]", acts.join(",")));

    facade
        .expect_upsert_activity()
        .times(100)
        .returning(|act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });

    let summary = super::process(UserId::from(1), &facade, &mut store)
        .await
        .unwrap();

    assert_eq!(summary.activities.len(), 100);
    let times = set_times.lock().unwrap().clone();
    assert_eq!(
        times,
        vec![cursor - 86400],
        "the persisted cursor steps to the start day minus one (no sentinel resolution)"
    );
}

// ─── accept_event: APP_SCOPE_CHANGED ────────────────────────────────────────

/// A raw intervals.icu webhook body with the given `secret` and events JSON
/// array elements.
fn icu_body(secret: &str, events: &str) -> String {
    format!(r#"{{"secret":"{secret}","events":[{events}]}}"#)
}

/// One `APP_SCOPE_CHANGED` event element (the live wire shape: `client_id`
/// and `client_name` are unmodeled — they land in the `_extra` catch-all).
fn scope_changed_json(athlete: &str, deauthorized: bool, scope: &str) -> String {
    format!(
        r#"{{"athlete_id":"{athlete}","type":"APP_SCOPE_CHANGED","timestamp":"2024-12-06T06:40:47.011+00:00","client_id":1714,"client_name":"tendabike","deauthorized":{deauthorized},"scope":"{scope}"}}"#
    )
}

/// An `APP_SCOPE_CHANGED` delivery for a **connected** user disables the user
/// (their queued events dropped, their token cleared) — the event is not
/// queued and adds nothing to the wake list.
#[tokio::test]
async fn accept_event_scope_change_disables_a_known_athlete() {
    let secret = icu_webhook_secret();
    let (mut store, _facade) = setup();
    // The athlete is resolved to a connected user (the lookup runs once).
    store
        .expect_icuser_get_by_icuid()
        .times(1)
        .returning(|_id| Ok(Some(icu_user(UserId::from(1), "a1", true))));
    // The disable: the queued events are dropped and the token is cleared.
    store
        .expect_icu_events_delete_for_user()
        .times(1)
        .returning(|_id| Ok(2));
    store
        .expect_icuser_update_token()
        .times(1)
        .returning(|_id, token| {
            assert!(token.is_none(), "the scope change clears the token");
            Ok(icu_user(UserId::from(1), "a1", false))
        });
    // Not queued: no `icu_event_create`.
    store.expect_icu_event_create().times(0);

    let body = icu_body(&secret, &scope_changed_json("a1", true, ""));
    let queued = accept_event(&body, &mut store).await.unwrap();
    assert!(queued.is_empty(), "a scope change wakes no user");
}

/// An `APP_SCOPE_CHANGED` delivery for an **unknown** athlete is a logged
/// no-op: no error, no disable, nothing queued.
#[tokio::test]
async fn accept_event_scope_change_is_a_noop_for_an_unknown_athlete() {
    let secret = icu_webhook_secret();
    let (mut store, _facade) = setup();
    // The athlete is looked up (once) and not found.
    store
        .expect_icuser_get_by_icuid()
        .times(1)
        .returning(|_id| Ok(None));
    // No disable: the queue drop and the token clear never run.
    store.expect_icu_events_delete_for_user().times(0);
    store.expect_icuser_update_token().times(0);
    store.expect_icu_event_create().times(0);

    let body = icu_body(
        &secret,
        &scope_changed_json("i-unknown", false, "athlete.read"),
    );
    let queued = accept_event(&body, &mut store).await.unwrap();

    assert!(queued.is_empty(), "an unknown athlete wakes no user");
}
