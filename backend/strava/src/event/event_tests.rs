use super::*;
use crate::test_support::{TestStravaSession, TestStravaStore, activity_json, strava_user};

fn setup() -> (TestStravaStore, TestStravaSession) {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let session = TestStravaSession::new(UserId::from(1), 42.into());
    (store, session)
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

fn activity_event() -> Event {
    Event {
        object_type: ObjectType::Activity,
        object_id: 10,
        aspect_type: AspectType::Create,
        owner_id: 42.into(),
        subscription_id: 1,
        event_time: 100,
        ..Default::default()
    }
}

#[tokio::test]
async fn into_event_maps_fields() -> TbResult<()> {
    let (mut store, _) = setup();
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
    let res = in_event("activity", 10, "create", 99, serde_json::Map::new())
        .into_event(&mut store)
        .await;
    assert!(matches!(res, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn into_event_rejects_unknown_types() {
    let (mut store, _) = setup();
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
    assert!(
        in_event("activity", 10, "create", 42, serde_json::Map::new())
            .accept(&mut store)
            .await?
            .is_some()
    );
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].object_id, 10);
    assert!(store.events[0].id.is_some());
    Ok(())
}

#[tokio::test]
async fn accept_athlete_deauth_disables_user() -> TbResult<()> {
    let (mut store, _) = setup();
    store
        .strava_event_create(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    let mut updates = serde_json::Map::new();
    updates.insert("authorized".into(), serde_json::json!("false"));
    assert!(
        in_event("athlete", 42, "update", 42, updates)
            .accept(&mut store)
            .await?
            .is_none()
    );
    assert_eq!(store.event_count(), 0);
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn insert_sync_stores_event() -> TbResult<()> {
    let (mut store, _) = setup();
    insert_sync(42.into(), 100, true, &mut store).await?;
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].object_type, ObjectType::Sync);
    assert_eq!(store.events[0].object_id, 1);
    Ok(())
}

#[tokio::test]
async fn insert_sync_rejects_future() {
    let (mut store, _) = setup();
    let res = insert_sync(42.into(), get_time() + 1000, false, &mut store).await;
    assert!(matches!(res, Err(Error::BadRequest(_))));
    assert_eq!(store.event_count(), 0);
}

#[tokio::test]
async fn insert_stop_schedules_future() -> TbResult<()> {
    let (mut store, _) = setup();
    insert_stop(&mut store).await?;
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].object_type, ObjectType::Stop);
    assert!(store.events[0].object_id > get_time());
    Ok(())
}

#[tokio::test]
async fn insert_stop_extends_existing() -> TbResult<()> {
    // A second 429 while a Stop is queued extends the deadline rather
    // than stacking rows (spec #446 §4.6): the backoff must not compound.
    let (mut store, _) = setup();
    insert_stop(&mut store).await?;
    let first_until = store.events[0].object_id;
    insert_stop(&mut store).await?;
    assert_eq!(
        store.event_count(),
        1,
        "the second Stop extends, it does not stack"
    );
    let stop = &store.events[0];
    assert_eq!(stop.object_type, ObjectType::Stop);
    assert!(
        stop.object_id >= first_until,
        "the deadline is never shortened"
    );
    assert!(
        stop.object_id > get_time(),
        "the extended Stop is still active"
    );
    Ok(())
}

#[tokio::test]
async fn get_event_returns_latest_and_drops_older() -> TbResult<()> {
    let (mut store, session) = setup();
    let mut first = activity_event();
    first.aspect_type = AspectType::Update;
    first.event_time = 50;
    store.strava_event_create(first).await?;
    store.strava_event_create(activity_event()).await?;
    let event = match get_event(&session, &mut store).await? {
        QueueRead::Event(event) => event,
        other => panic!("the latest event is returned, got {other:?}"),
    };
    assert_eq!(event.aspect_type, AspectType::Create);
    assert_eq!(store.event_count(), 1);
    Ok(())
}

#[tokio::test]
async fn get_event_ignores_other_owners() -> TbResult<()> {
    let (mut store, session) = setup();
    store
        .strava_event_create(Event {
            owner_id: 43.into(),
            ..activity_event()
        })
        .await?;
    assert_eq!(get_event(&session, &mut store).await?, QueueRead::Empty);
    Ok(())
}

#[tokio::test]
async fn get_event_active_stop_reports_deadline() -> TbResult<()> {
    // An unexpired global Stop pauses the queue: the read reports the
    // deadline (the caller sleeps until it) and consumes nothing.
    let (mut store, session) = setup();
    let until = get_time() + 500;
    store
        .strava_event_create(Event {
            object_type: ObjectType::Stop,
            object_id: until,
            ..Default::default()
        })
        .await?;
    store.strava_event_create(activity_event()).await?;
    let res = get_event(&session, &mut store).await?;
    assert_eq!(res, QueueRead::Limited { until });
    assert_eq!(store.event_count(), 2);
    Ok(())
}

#[tokio::test]
async fn get_event_expired_stop_deletes_and_returns_next() -> TbResult<()> {
    // An expired Stop is deleted by the read, which recurses for the next
    // event (the backoff is over).
    let (mut store, session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Stop,
            object_id: 100,
            ..Default::default()
        })
        .await?;
    store.strava_event_create(activity_event()).await?;
    let res = get_event(&session, &mut store).await?;
    assert!(matches!(res, QueueRead::Event(ref e) if e.object_id == 10));
    assert_eq!(store.event_count(), 1);
    Ok(())
}

#[tokio::test]
async fn rate_limit_active_reports_deadline() -> TbResult<()> {
    let (mut store, session) = setup();
    let until = get_time() + 500;
    store
        .strava_event_create(Event {
            object_type: ObjectType::Stop,
            object_id: until,
            ..Default::default()
        })
        .await?;
    let event = store.events[0].clone();
    let res = event.rate_limit(&session, &mut store).await?;
    assert_eq!(res, QueueRead::Limited { until });
    assert_eq!(store.event_count(), 1);
    Ok(())
}

#[tokio::test]
async fn rate_limit_expired_removes_stop() -> TbResult<()> {
    let (mut store, session) = setup();
    let stop = Event {
        object_type: ObjectType::Stop,
        object_id: 100,
        ..Default::default()
    };
    store.strava_event_create(stop).await?;
    let event = store.events[0].clone();
    let res = event.rate_limit(&session, &mut store).await?;
    assert_eq!(res, QueueRead::Empty);
    assert_eq!(store.event_count(), 0);
    Ok(())
}

#[tokio::test]
async fn process_no_events() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    Ok(())
}

#[tokio::test]
async fn process_activity_create_imports() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store.strava_event_create(activity_event()).await?;
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.get(&ActivityId::new(10)).is_some());
    assert_eq!(store.event_count(), 0);
    Ok(())
}

#[tokio::test]
async fn process_activity_delete_missing_ignores() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            aspect_type: AspectType::Delete,
            ..activity_event()
        })
        .await?;
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 0);
    Ok(())
}

#[tokio::test]
async fn process_try_again_inserts_stop() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store.strava_event_create(activity_event()).await?;
    session.queue_error("/activities/10", Error::TryAgain("rate limit"));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 2);
    let stop = store
        .events
        .iter()
        .find(|e| e.object_type == ObjectType::Stop)
        .unwrap();
    assert!(stop.object_id > get_time());
    Ok(())
}

#[tokio::test]
async fn process_activity_nontransient_drops_event() -> TbResult<()> {
    // A permanently bad activity (parse error, a 404's domain shape) is
    // dropped from the queue and logged — it is committed, so the event
    // is gone and the queue cannot wedge on it (spec #446 §4.6).
    let (mut store, mut session) = setup();
    store.strava_event_create(activity_event()).await?;
    session.queue_error(
        "/activities/10",
        Error::AnyFailure(anyhow::anyhow!("parse boom")),
    );
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 0, "the poison event is deleted");
    Ok(())
}

#[tokio::test]
async fn process_activity_not_auth_disables_user() -> TbResult<()> {
    // A 401 after a token refresh disables the user's Strava integration
    // and drops all their queued events (spec #446 §4.6): the data freezes
    // stale-but-consistent; no event-by-event 401s follow.
    let (mut store, mut session) = setup();
    store.strava_event_create(activity_event()).await?;
    session.queue_error("/activities/10", Error::NotAuth(String::from("token dead")));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(
        store.event_count(),
        0,
        "the user's queued events are dropped"
    );
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert!(user.disabled(), "the StravaId is disabled");
    Ok(())
}

#[tokio::test]
async fn process_sync_not_auth_disables_user() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Sync,
            owner_id: 42.into(),
            ..Default::default()
        })
        .await?;
    session.queue_error(
        "/activities?after=0&per_page=25",
        Error::NotAuth(String::from("token dead")),
    );
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 0);
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn process_sync_empty_list_removes_event() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Sync,
            owner_id: 42.into(),
            ..Default::default()
        })
        .await?;
    session.queue("/activities?after=0&per_page=25", "[]");
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 0);
    Ok(())
}

#[tokio::test]
async fn process_sync_imports_activities() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Sync,
            owner_id: 42.into(),
            ..Default::default()
        })
        .await?;
    session.queue(
        "/activities?after=0&per_page=25",
        &format!("[{}]", activity_json(10, "Ride", None)),
    );
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].event_time, 1767348000);
    let acts = ActivityStore::get_all(&mut store.mem, &UserId::from(1)).await?;
    assert_eq!(acts.len(), 1);
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
    store
        .strava_event_create(Event {
            object_type: ObjectType::Sync,
            owner_id: 42.into(),
            ..Default::default()
        })
        .await?;
    session.queue(
        "/activities?after=0&per_page=25",
        &format!(
            "[{}, {}]",
            activity_json(10, "Ride", None),
            activity_json(11, "Ride", Some("g11"))
                .replace("2026-01-02T10:00:00Z", "2026-01-03T10:00:00Z")
        ),
    );
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    session.queue_error(
        "/gear/g11",
        Error::AnyFailure(anyhow::anyhow!("parse boom")),
    );
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1, "the good activity imports");
    assert_eq!(store.event_count(), 1, "the sync event stays queued");
    assert_eq!(
        store.events[0].event_time, 1767434400,
        "the cursor advances past the bad activity (2026-01-03T10:00:00Z)"
    );
    Ok(())
}

#[tokio::test]
async fn process_sync_try_again_keeps_event_and_stops() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Sync,
            owner_id: 42.into(),
            ..Default::default()
        })
        .await?;
    session.queue_error("/activities?after=0&per_page=25", Error::TryAgain("nope"));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 2);
    assert!(
        store
            .events
            .iter()
            .any(|e| e.object_type == ObjectType::Stop)
    );
    Ok(())
}

#[tokio::test]
async fn process_skips_athlete_event() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Athlete,
            object_id: 42,
            ..Default::default()
        })
        .await?;
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 0);
    Ok(())
}

#[tokio::test]
async fn sync_users_skips_disabled() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    UserStore::create(&mut store.mem, "A", "User", &None).await?;
    UserStore::create(&mut store.mem, "B", "User", &None).await?;
    store.insert_user(strava_user(UserId::from(1), 42, true));
    store.insert_user(strava_user(UserId::from(2), 43, false));
    sync_users(None, 100, false, &mut store).await?;
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].owner_id, 42.into());
    Ok(())
}

#[tokio::test]
async fn sync_users_single() -> TbResult<()> {
    let (mut store, _) = setup();
    sync_users(Some(UserId::from(1)), 100, false, &mut store).await?;
    assert_eq!(store.event_count(), 1);
    Ok(())
}

#[tokio::test]
async fn sync_users_unknown_user() {
    let (mut store, _) = setup();
    let res = sync_users(Some(UserId::from(99)), 100, false, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}
