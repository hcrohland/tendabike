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
