use super::super::test_support::{
    activity_json, icu_user, queue, queue_error, setup, stub_activity_json,
};
use super::*;
use crate::facade::{
    ActivityId, Notification, NotificationKind, NotificationLifecycle, NotificationState,
    NotificationSubject, Summary,
};
use time::OffsetDateTime;

fn event(id: i32, activity: &str) -> IcuEvent {
    IcuEvent {
        id: Some(id),
        icu_athlete_id: "a1".into(),
        activity_id: activity.to_string(),
        event_time: 1000,
        object_type: "ACTIVITY".to_string(),
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
        message: Some("2".to_string()),
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

// ─── get_event (probe read) ─────────────────────────────────────────────────

#[tokio::test]
async fn get_event_returns_queued_event() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(event(1, "10"))));
    let read = get_event(UserId::from(1), &mut store).await.unwrap();
    assert!(matches!(read, QueueRead::Event(_)));
}

#[tokio::test]
async fn get_event_empty_when_no_events() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(None));
    let read = get_event(UserId::from(1), &mut store).await.unwrap();
    assert!(matches!(read, QueueRead::Empty));
}

#[tokio::test]
async fn get_event_limited_when_rate_limited() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_limited();
    // The rate-limit gate fires before any store read.
    let read = get_event(UserId::from(1), &mut store).await.unwrap();
    assert!(matches!(read, QueueRead::Limited { .. }));
}

// ─── process (batch drain) ──────────────────────────────────────────────────

#[tokio::test]
async fn process_drains_queue_and_imports() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    // Two events, then empty.
    store.expect_icu_event_get_next_for_user().returning(|_id| {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(match n {
            0 => Some(event(1, "10")),
            1 => Some(event(2, "11")),
            _ => None,
        })
    });
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    queue("/activity/10", &activity_json("10", "Ride", None));
    queue("/activity/11", &activity_json("11", "Ride", None));
    facade
        .expect_upsert_activity()
        .times(2)
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    assert_eq!(summary.activities.len(), 2);
    assert!(summary.notifications.is_empty());
}

#[tokio::test]
async fn process_stub_batch_emits_one_notice() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    store.expect_icu_event_get_next_for_user().returning(|_id| {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(match n {
            0 => Some(event(1, "10")),
            1 => Some(event(2, "11")),
            _ => None,
        })
    });
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    queue("/activity/10", &stub_activity_json("10", "Ride"));
    queue("/activity/11", &stub_activity_json("11", "Ride"));
    // The one batch notice: the facade's create_notification returns it.
    facade
        .expect_create_notification()
        .returning(|_r, _l, _k, _s, message, _store| {
            assert_eq!(message.as_deref(), Some("2"), "the skip count is 2");
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
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(event(1, "10"))));
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
async fn process_429_stops_and_keeps_event_queued() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(event(1, "10"))));
    // The 429 stops the batch: the event is NOT deleted (it stays queued).
    queue_error("/activity/10", Error::TryAgain("rate limited"));
    let summary = process(UserId::from(1), &_facade, &mut store)
        .await
        .unwrap();
    assert!(summary.is_empty());
}

#[tokio::test]
async fn process_permanent_error_drops_event_and_continues() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    // Event 1 is poison (404), event 2 is good.
    store.expect_icu_event_get_next_for_user().returning(|_id| {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(match n {
            0 => Some(event(1, "10")),
            1 => Some(event(2, "11")),
            _ => None,
        })
    });
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    queue_error("/activity/10", Error::NotFound("gone".to_string()));
    queue("/activity/11", &activity_json("11", "Ride", None));
    facade
        .expect_upsert_activity()
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    // Only the good activity (event 2) is imported.
    assert_eq!(summary.activities.len(), 1);
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
    // One ACTIVITY_DELETED event, then empty.
    store.expect_icu_event_get_next_for_user().returning(|_id| {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(match n {
            0 => Some(deleted_event(1, "10")),
            _ => None,
        })
    });
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

#[tokio::test]
async fn process_mixed_batch_imports_and_deletes() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
    // An upload (import) followed by a delete.
    store.expect_icu_event_get_next_for_user().returning(|_id| {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(match n {
            0 => Some(event(1, "10")),
            1 => Some(deleted_event(2, "11")),
            _ => None,
        })
    });
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    queue("/activity/10", &activity_json("10", "Ride", None));
    facade
        .expect_upsert_activity()
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    facade
        .expect_delete_activity()
        .returning(|id, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.tombstone(id);
            Ok(summary)
        });
    let summary = process(UserId::from(1), &facade, &mut store).await.unwrap();
    // The uploaded activity is live; the deleted one is a tombstone.
    assert_eq!(summary.activities.len(), 2);
    assert!(
        summary
            .activities
            .get(&ActivityId::from("icu:10"))
            .is_some()
    );
    assert!(
        summary
            .activities
            .is_tombstoned(&ActivityId::from("icu:11"))
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
        object_type: "SYNC".to_string(),
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
    assert_eq!(events[0].object_type, "SYNC");
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

/// `process` on a `SYNC` event resolves the `0` sentinel to now, walks
/// `[oldest, newest]`, and on a `< 100` batch deletes the row and returns the
/// union `Summary`.
#[tokio::test]
async fn process_sync_walks_and_deletes_on_completion() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    // The queue holds one `SYNC` row (the `0` sentinel, full history).
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(sync_event(1, 0, 0))));
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
        "the walk's imported activities ride the returned Summary"
    );
}

/// `process` on a `SYNC` event that 429s mid-walk: the cursor is *not*
/// advanced past the 429 and the row is *not* deleted (a later `process`
/// resumes from the stored cursor).
#[tokio::test]
async fn process_sync_429_keeps_cursor_and_row() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(sync_event(1, 0, 0))));
    // The zero-sentinel resolution persists `now` (the cursor is set before
    // the first fetch). No `icu_event_delete` is expected: the 429 stops the
    // walk before completion, so the row stays queued.
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
        "a 429 mid-walk imports nothing and leaves the row queued"
    );
}

/// `process` on a `SYNC` event where the first batch succeeds (the cursor
/// advances) and the second fetch 429s: the persisted cursor equals the first
/// batch's oldest activity (not advanced further) and the `SYNC` row remains.
#[tokio::test]
async fn process_sync_429_after_first_batch_keeps_advanced_cursor_and_row() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    // The queue holds one `SYNC` row (the `0` sentinel, full history).
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(sync_event(1, 0, 0))));
    // The zero-sentinel resolution + the first batch's cursor advance are
    // persisted. No `icu_event_delete` is expected: the 429 on the second
    // fetch stops the walk before completion, so the row stays queued.
    let set_times: std::sync::Arc<std::sync::Mutex<Vec<i64>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = set_times.clone();
    store.expect_icu_event_set_time().returning(move |_id, t| {
        rec.lock().unwrap().push(t);
        Ok(())
    });
    // A full batch (100) → the cursor advances to the oldest activity.
    let acts: Vec<String> = (0..100)
        .map(|i| activity_json(&format!("100{}", i), "Ride", None))
        .collect();
    queue(&sync_uri(0, now_ts()), &format!("[{}]", acts.join(",")));
    // The oldest activity in the first batch (all share `activity_json`'s
    // start date, 2026-01-02T10:00:00Z).
    use time::format_description::well_known::Rfc3339;
    let oldest_ts = OffsetDateTime::parse("2026-01-02T10:00:00Z", &Rfc3339)
        .unwrap()
        .unix_timestamp();
    // The second fetch (bounded by the day before the oldest activity) 429s.
    queue_error(
        &sync_uri(0, oldest_ts - 86400),
        Error::TryAgain("rate limited"),
    );

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
        "the first batch's activities are imported"
    );
    let times = set_times.lock().unwrap().clone();
    // The first `set_time` is the zero-sentinel resolution (now); the second
    // is the cursor advance to the oldest activity in the first batch. The
    // 429 on the second fetch stops the walk, so the cursor is not advanced
    // further.
    assert_eq!(
        times.len(),
        2,
        "the sentinel resolution and the one cursor advance (no further advance on the 429)"
    );
    assert_eq!(
        times[1], oldest_ts,
        "the persisted cursor equals the first batch's oldest activity"
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
    store
        .expect_icu_event_get_next_for_user()
        .returning(move |_id| Ok(Some(sync_event(1, 0, cursor))));
    // The stored cursor is non-zero: the sentinel is NOT re-resolved, so
    // `icu_event_set_time` is only called for the (later) cursor advance.
    store
        .expect_icu_event_set_time()
        .returning(|_id, _t| Ok(()));
    store.expect_icu_event_delete().returning(|_id| Ok(()));
    // The first batch is bounded by the stored cursor (not now).
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
        "the walk resumes from the stored cursor"
    );
    // The first fetch used the stored cursor (one day before now), not now.
    let reqs = super::super::test_support::requests();
    assert_eq!(reqs.len(), 1, "exactly one batch is fetched");
    assert!(
        reqs[0].contains(&format!("newest={}", super::date_str(cursor))),
        "the first fetch is bounded by the stored cursor"
    );
}

/// `process` on a `SYNC` event advances the persisted `newest` cursor to the
/// oldest activity in a full (100) batch, then continues from the day before.
#[tokio::test]
async fn process_sync_advances_cursor_per_batch() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    sync_user_fixture(&mut store);
    store
        .expect_icu_event_get_next_for_user()
        .returning(|_id| Ok(Some(sync_event(1, 0, 0))));
    // The zero-sentinel resolution + the per-batch cursor advance.
    let set_times: std::sync::Arc<std::sync::Mutex<Vec<i64>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = set_times.clone();
    store.expect_icu_event_set_time().returning(move |_id, t| {
        rec.lock().unwrap().push(t);
        Ok(())
    });
    store.expect_icu_event_delete().returning(|_id| Ok(()));

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
    // The next batch is bounded by the day before the oldest activity
    // (< 100 → end of history).
    queue(&sync_uri(0, oldest_ts - 86400), "[]");

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
    // The first `set_time` is the zero-sentinel resolution (now); the second
    // is the cursor advance to the oldest activity in the full batch.
    assert!(
        times.len() >= 2,
        "the sentinel resolution and the cursor advance"
    );
    assert_eq!(
        times[1], oldest_ts,
        "the persisted cursor equals the oldest activity in the full batch"
    );
}
