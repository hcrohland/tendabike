use std::sync::{Arc, Mutex};

use super::*;
use crate::event::{AspectType, Event, ObjectType};
use crate::test_support::{
    MockStravaConn, activity, activity_json, queue, queue_error, strava_admin_session,
    strava_session, strava_user,
};
use tb_domain::MockSession;

fn setup() -> (MockStravaConn, MockSession) {
    let store = MockStravaConn::new();
    let session = strava_session(UserId::from(1), 42.into());
    (store, session)
}

/// Sets the global rate limiter to "at limit" (read 15-min exhausted).
fn set_rate_limited() {
    crate::ratelimit::test_set_limited();
}

/// Resets the global rate limiter to "no data" (allows all requests).
fn reset_rate_limiter() {
    crate::ratelimit::test_set_not_limited();
}

fn in_event(
    object_type: &str,
    object_id: i64,
    aspect_type: &str,
    owner: i32,
    updates: serde_json::Map<String, serde_json::Value>,
) -> InEvent {
    serde_json::from_value(serde_json::json!({
        "object_type": object_type,
        "object_id": object_id,
        "aspect_type": aspect_type,
        "updates": updates,
        "owner_id": owner,
        "subscription_id": 1,
        "event_time": 100
    }))
    .unwrap()
}

/// A queued event with an explicit id (the id the delete/batch-drop assertions
/// key on).
fn event_with_id(
    id: i32,
    object_type: ObjectType,
    object_id: i64,
    aspect: AspectType,
    event_time: i64,
) -> Event {
    Event {
        id: Some(id),
        object_type,
        object_id,
        aspect_type: aspect,
        owner_id: 42.into(),
        subscription_id: 1,
        event_time,
        ..Default::default()
    }
}

/// Canned the `get_event` read of a single queued event: the event is both the
/// "next" event and the only "later" event for its object.
fn queue_event(store: &mut MockStravaConn, event: Event) {
    let next = event.clone();
    let later = event.clone();
    store
        .expect_strava_event_get_next_for_user()
        .returning(move |_user| Ok(Some(next.clone())));
    store
        .expect_strava_event_get_later()
        .returning(move |_obj_id, _oid| Ok(vec![later.clone()]));
}

#[tokio::test]
async fn into_event_maps_fields() -> TbResult<()> {
    let (mut store, _) = setup();
    // The owner (42) is a known strava user: the event maps cleanly.
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(Some(strava_user(UserId::from(1), 42, true))));
    let event = in_event("activity", 10, "create", 42, serde_json::Map::new())
        .into_event(&mut store)
        .await?;
    assert!(event.id.is_none());
    assert_eq!(event.object_type, ObjectType::Activity);
    assert_eq!(event.object_id, 10);
    assert_eq!(event.aspect_type, AspectType::Create);
    assert_eq!(event.owner_id, 42.into());
    assert_eq!(event.subscription_id, 1);
    assert_eq!(event.event_time, 100);
    Ok(())
}

#[tokio::test]
async fn into_event_rejects_unknown_owner() {
    let (mut store, _) = setup();
    // An unknown owner (99) is a `BadRequest`.
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(None));
    let res = in_event("activity", 10, "create", 99, serde_json::Map::new())
        .into_event(&mut store)
        .await;
    assert!(matches!(res, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn into_event_rejects_unknown_types() {
    let (mut store, _) = setup();
    // The owner is known; the object/aspect types are not.
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(Some(strava_user(UserId::from(1), 42, true))));
    let bad_object = in_event("flying", 10, "create", 42, serde_json::Map::new());
    assert!(matches!(
        bad_object.into_event(&mut store).await,
        Err(Error::BadRequest(_))
    ));
    let bad_aspect = in_event("activity", 10, "explode", 42, serde_json::Map::new());
    assert!(matches!(
        bad_aspect.into_event(&mut store).await,
        Err(Error::BadRequest(_))
    ));
}

#[tokio::test]
async fn accept_stores_activity_event() -> TbResult<()> {
    let (mut store, _) = setup();
    // An activity event is queued: the owner is known (twice: `accept` and
    // `into_event`), and the event is stored.
    let created = Arc::new(Mutex::new(None));
    let created_rec = created.clone();
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(Some(strava_user(UserId::from(1), 42, true))));
    store.expect_strava_event_create().returning(move |e| {
        created_rec.lock().unwrap().replace(e.clone());
        Ok(())
    });
    assert_eq!(
        in_event("activity", 10, "create", 42, serde_json::Map::new())
            .accept(&mut store)
            .await?,
        Some(UserId::from(1))
    );
    let event = created
        .lock()
        .unwrap()
        .take()
        .expect("the event was queued");
    assert_eq!(event.object_id, 10);
    assert_eq!(event.object_type, ObjectType::Activity);
    Ok(())
}

#[tokio::test]
async fn accept_athlete_deauth_disables_user() -> TbResult<()> {
    let (mut store, _) = setup();
    // An athlete deauth (`authorized: false`) disables the user: drops their
    // events and clears their token. Nothing is queued.
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(Some(strava_user(UserId::from(1), 42, true))));
    let events_deleted = Arc::new(Mutex::new(0));
    let events_deleted_rec = events_deleted.clone();
    let token_refresh = Arc::new(Mutex::new(Vec::new()));
    let token_refresh_rec = token_refresh.clone();
    store
        .expect_strava_events_delete_for_user()
        .returning(move |_user| {
            *events_deleted_rec.lock().unwrap() = 1;
            Ok(1)
        });
    store
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            token_refresh_rec.lock().unwrap().push(refresh);
            Ok(strava_user(UserId::from(1), 42, false))
        });
    let mut updates = serde_json::Map::new();
    updates.insert("authorized".into(), serde_json::json!("false"));
    assert_eq!(
        in_event("athlete", 42, "update", 42, updates)
            .accept(&mut store)
            .await?,
        None
    );
    assert_eq!(
        *events_deleted.lock().unwrap(),
        1,
        "the user's events are dropped"
    );
    assert_eq!(
        token_refresh.lock().unwrap().as_slice(),
        [None],
        "the token is cleared (the user is disabled)"
    );
    Ok(())
}

#[tokio::test]
async fn insert_sync_stores_event() -> TbResult<()> {
    let (mut store, _) = setup();
    // A migrate sync queues a `Sync` event with `object_id` 1 for the user's
    // strava id (42, from the warm token cache).
    let created = Arc::new(Mutex::new(None));
    let created_rec = created.clone();
    store.expect_strava_event_create().returning(move |e| {
        created_rec.lock().unwrap().replace(e.clone());
        Ok(())
    });
    insert_sync(UserId::from(1), 100, true, &mut store).await?;
    let event = created.lock().unwrap().take().expect("the sync was queued");
    assert_eq!(event.object_type, ObjectType::Sync);
    assert_eq!(event.object_id, 1);
    assert_eq!(event.owner_id, 42.into());
    Ok(())
}

#[tokio::test]
async fn insert_sync_rejects_future() {
    let (mut store, _) = setup();
    // A future event time is a `BadRequest` (no store write fires).
    let res = insert_sync(UserId::from(1), get_time() + 1000, false, &mut store).await;
    assert!(matches!(res, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn get_event_returns_latest_and_drops_older() -> TbResult<()> {
    let (mut store, session) = setup();
    // Two events for the same object: the read returns the latest (Create)
    // and drops the older (Update).
    let first = event_with_id(1, ObjectType::Activity, 10, AspectType::Update, 50);
    let second = event_with_id(2, ObjectType::Activity, 10, AspectType::Create, 100);
    let next = first.clone();
    let later_first = first.clone();
    let later_second = second.clone();
    store
        .expect_strava_event_get_next_for_user()
        .returning(move |_user| Ok(Some(next.clone())));
    store
        .expect_strava_event_get_later()
        .returning(move |_obj_id, _oid| Ok(vec![later_first.clone(), later_second.clone()]));
    let dropped = Arc::new(Mutex::new(Vec::new()));
    let dropped_rec = dropped.clone();
    store
        .expect_strava_events_delete_batch()
        .returning(move |values| {
            dropped_rec.lock().unwrap().extend(values);
            Ok(())
        });
    let event = match get_event(session.user_id(), &mut store).await? {
        QueueRead::Event(event) => event,
        other => panic!("the latest event is returned, got {other:?}"),
    };
    assert_eq!(event.aspect_type, AspectType::Create);
    assert_eq!(
        dropped.lock().unwrap().as_slice(),
        [Some(1)],
        "the older event is dropped"
    );
    Ok(())
}

#[tokio::test]
async fn get_event_ignores_other_owners() -> TbResult<()> {
    let (mut store, session) = setup();
    // The queue has no event for this user's strava id: the read is `Empty`.
    store
        .expect_strava_event_get_next_for_user()
        .returning(|_user| Ok(None));
    assert_eq!(
        get_event(session.user_id(), &mut store).await?,
        QueueRead::Empty
    );
    Ok(())
}

#[tokio::test]
async fn get_event_rate_limited_returns_limited() -> TbResult<()> {
    // When the rate limiter reports no budget, get_event returns Limited
    // without dequeuing: the event stays queued for when the window resets.
    set_rate_limited();
    let (mut _store, session) = setup();
    let res = get_event(session.user_id(), &mut _store).await?;
    assert!(matches!(res, QueueRead::Limited { .. }));
    reset_rate_limiter();
    Ok(())
}

#[tokio::test]
async fn get_event_not_rate_limited_returns_event() -> TbResult<()> {
    // When the rate limiter has budget, get_event dequeues normally.
    reset_rate_limiter();
    let (mut store, session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Activity, 10, AspectType::Create, 100),
    );
    let res = get_event(session.user_id(), &mut store).await?;
    assert!(matches!(res, QueueRead::Event(ref e) if e.object_id == 10));
    Ok(())
}

#[tokio::test]
async fn process_no_events() -> TbResult<()> {
    let (mut store, mut session) = setup();
    // An empty queue: nothing to process.
    store
        .expect_strava_event_get_next_for_user()
        .returning(|_user| Ok(None));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    Ok(())
}

#[tokio::test]
async fn process_activity_create_imports() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Activity, 10, AspectType::Create, 100),
    );
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The activity is new (read `None`, create), then the event is consumed.
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    let deleted = Arc::new(Mutex::new(None));
    let deleted_rec = deleted.clone();
    store.expect_strava_event_delete().returning(move |id| {
        deleted_rec.lock().unwrap().replace(id);
        Ok(())
    });
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.get(&ActivityId::new(10)).is_some());
    assert_eq!(
        deleted.lock().unwrap().take(),
        Some(Some(1)),
        "the event is consumed"
    );
    Ok(())
}

#[tokio::test]
async fn process_activity_delete_missing_ignores() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Activity, 10, AspectType::Delete, 100),
    );
    // A delete for a missing activity is ignored (the event is consumed).
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    store.expect_strava_event_delete().returning(|_id| Ok(()));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    Ok(())
}

#[tokio::test]
async fn process_try_again_keeps_event_queued() -> TbResult<()> {
    // A 429 (TryAgain) keeps the event queued: the rate-limit state is
    // updated from the response headers (in production), and the next
    // get_event probe will report Limited. No Stop event is created.
    reset_rate_limiter();
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Activity, 10, AspectType::Create, 100),
    );
    // The 429 keeps the event queued: it is NOT deleted.
    store.expect_strava_event_delete().times(0);
    queue_error("/activities/10", Error::TryAgain("rate limit"));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    Ok(())
}

#[tokio::test]
async fn process_activity_nontransient_drops_event() -> TbResult<()> {
    // A permanently bad activity (parse error, a 404's domain shape) is
    // dropped from the queue and logged — it is committed, so the event
    // is gone and the queue cannot wedge on it (spec #446 §4.6).
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Activity, 10, AspectType::Create, 100),
    );
    let deleted = Arc::new(Mutex::new(None));
    let deleted_rec = deleted.clone();
    store.expect_strava_event_delete().returning(move |id| {
        deleted_rec.lock().unwrap().replace(id);
        Ok(())
    });
    queue_error(
        "/activities/10",
        Error::AnyFailure(anyhow::anyhow!("parse boom")),
    );
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(
        deleted.lock().unwrap().take(),
        Some(Some(1)),
        "the poison event is deleted"
    );
    Ok(())
}

#[tokio::test]
async fn process_activity_not_auth_disables_user() -> TbResult<()> {
    // A 401 after a token refresh disables the user's Strava integration
    // and drops all their queued events (spec #446 §4.6): the data freezes
    // stale-but-consistent; no event-by-event 401s follow.
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Activity, 10, AspectType::Create, 100),
    );
    let token_refresh = Arc::new(Mutex::new(Vec::new()));
    let token_refresh_rec = token_refresh.clone();
    store
        .expect_strava_events_delete_for_user()
        .returning(|_user| Ok(1));
    store
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            token_refresh_rec.lock().unwrap().push(refresh);
            Ok(strava_user(UserId::from(1), 42, false))
        });
    queue_error("/activities/10", Error::NotAuth(String::from("token dead")));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(
        token_refresh.lock().unwrap().as_slice(),
        [None],
        "the user's token is cleared (the StravaId is disabled)"
    );
    Ok(())
}

#[tokio::test]
async fn process_sync_not_auth_disables_user() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Sync, 0, AspectType::Create, 0),
    );
    let token_refresh = Arc::new(Mutex::new(Vec::new()));
    let token_refresh_rec = token_refresh.clone();
    store
        .expect_strava_events_delete_for_user()
        .returning(|_user| Ok(1));
    store
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            token_refresh_rec.lock().unwrap().push(refresh);
            Ok(strava_user(UserId::from(1), 42, false))
        });
    queue_error(
        "/activities?after=0&per_page=25",
        Error::NotAuth(String::from("token dead")),
    );
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(
        token_refresh.lock().unwrap().as_slice(),
        [None],
        "the user's token is cleared (the StravaId is disabled)"
    );
    Ok(())
}

#[tokio::test]
async fn process_sync_empty_list_removes_event() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Sync, 0, AspectType::Create, 0),
    );
    // An empty activity list: the sync event is consumed.
    let deleted = Arc::new(Mutex::new(None));
    let deleted_rec = deleted.clone();
    store.expect_strava_event_delete().returning(move |id| {
        deleted_rec.lock().unwrap().replace(id);
        Ok(())
    });
    queue("/activities?after=0&per_page=25", "[]");
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(
        deleted.lock().unwrap().take(),
        Some(Some(1)),
        "the sync event is consumed"
    );
    Ok(())
}

#[tokio::test]
async fn process_sync_imports_activities() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Sync, 0, AspectType::Create, 0),
    );
    queue(
        "/activities?after=0&per_page=25",
        &format!("[{}]", activity_json(10, "Ride", None)),
    );
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The activity is new (read `None`, create); the sync event stays queued
    // with its cursor advanced to the activity's start time.
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    let cursor = Arc::new(Mutex::new(0));
    let cursor_rec = cursor.clone();
    store
        .expect_strava_event_set_time()
        .returning(move |_id, e_time| {
            *cursor_rec.lock().unwrap() = e_time;
            Ok(())
        });
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(
        *cursor.lock().unwrap(),
        1767348000,
        "the cursor advances to the activity's start time"
    );
    Ok(())
}

#[tokio::test]
async fn process_sync_bad_activity_skips_and_advances_cursor() -> TbResult<()> {
    // A permanently bad activity in a batch is skipped and the cursor
    // advances past it (spec #446 §4.6) — liveness > completeness: one bad
    // activity must not wedge the queue in an infinite retry loop. The
    // event stays queued at the advanced cursor (one page per wake); the
    // admin sync endpoint re-covers the gap if the user cares.
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Sync, 0, AspectType::Create, 0),
    );
    queue(
        "/activities?after=0&per_page=25",
        &format!(
            "[{}, {}]",
            activity_json(10, "Ride", None),
            activity_json(11, "Ride", Some("g11"))
                .replace("2026-01-02T10:00:00Z", "2026-01-03T10:00:00Z")
        ),
    );
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The good activity (10) is new (read `None`, create); the bad activity
    // (11, gear g11) fails its gear fetch and is skipped.
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    let cursor = Arc::new(Mutex::new(0));
    let cursor_rec = cursor.clone();
    store
        .expect_strava_event_set_time()
        .returning(move |_id, e_time| {
            *cursor_rec.lock().unwrap() = e_time;
            Ok(())
        });
    queue_error(
        "/gear/g11",
        Error::AnyFailure(anyhow::anyhow!("parse boom")),
    );
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1, "the good activity imports");
    assert_eq!(
        *cursor.lock().unwrap(),
        1767434400,
        "the cursor advances past the bad activity (2026-01-03T10:00:00Z)"
    );
    Ok(())
}

#[tokio::test]
async fn process_sync_try_again_keeps_event_queued() -> TbResult<()> {
    reset_rate_limiter();
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Sync, 0, AspectType::Create, 0),
    );
    // The 429 keeps the sync event queued: it is NOT deleted.
    store.expect_strava_event_delete().times(0);
    queue_error("/activities?after=0&per_page=25", Error::TryAgain("nope"));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    Ok(())
}

#[tokio::test]
async fn process_skips_athlete_event() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue_event(
        &mut store,
        event_with_id(1, ObjectType::Athlete, 42, AspectType::Update, 100),
    );
    // An athlete event is not processed: it is dropped (consumed).
    store.expect_strava_event_delete().returning(|_id| Ok(()));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    Ok(())
}

#[tokio::test]
async fn sync_users_skips_disabled() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    let mut session = strava_admin_session(UserId::from(1), 42.into());
    // The all-users sync queues a `Sync` event for every non-disabled user
    // (the disabled one is skipped).
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store.expect_stravauser_get_all().returning(|| {
        Ok(vec![
            strava_user(UserId::from(1), 42, true),
            strava_user(UserId::from(2), 43, false),
        ])
    });
    store.expect_strava_event_create().returning(move |e| {
        created_rec.lock().unwrap().push(e.clone());
        Ok(())
    });
    sync_users(&mut session, None, 100, false, &mut store).await?;
    let events = created.lock().unwrap().clone();
    assert_eq!(events.len(), 1, "only the enabled user is synced");
    assert_eq!(events[0].owner_id, 42.into());
    Ok(())
}

#[tokio::test]
async fn sync_users_single() -> TbResult<()> {
    let (mut store, _) = setup();
    let mut session = strava_admin_session(UserId::from(1), 42.into());
    // A single-user sync queues a `Sync` event for that user.
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store
        .expect_stravauser_get_by_tbid()
        .returning(|_id| Ok(strava_user(UserId::from(1), 42, true)));
    store.expect_strava_event_create().returning(move |e| {
        created_rec.lock().unwrap().push(e.clone());
        Ok(())
    });
    sync_users(&mut session, Some(UserId::from(1)), 100, false, &mut store).await?;
    assert_eq!(created.lock().unwrap().len(), 1);
    Ok(())
}

#[tokio::test]
async fn sync_users_unknown_user() {
    let (mut store, _) = setup();
    let mut session = strava_admin_session(UserId::from(1), 42.into());
    // A single-user sync for an unknown user is a `NotFound`.
    store.expect_stravauser_get_by_tbid().returning(|_id| {
        Err(Error::NotFound(
            "user not registered with strava".to_string(),
        ))
    });
    let res = sync_users(&mut session, Some(UserId::from(99)), 100, false, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}
