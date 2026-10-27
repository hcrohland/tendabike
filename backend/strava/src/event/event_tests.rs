use super::*;
use crate::test_support::{
    TestStravaStore, activity_json, queue, queue_error, strava_admin_session, strava_session,
    strava_user,
};
use tb_domain::MockSession;

fn setup() -> (TestStravaStore, MockSession) {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
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
    insert_sync(UserId::from(1), 100, true, &mut store).await?;
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].object_type, ObjectType::Sync);
    assert_eq!(store.events[0].object_id, 1);
    Ok(())
}

#[tokio::test]
async fn insert_sync_rejects_future() {
    let (mut store, _) = setup();
    let res = insert_sync(UserId::from(1), get_time() + 1000, false, &mut store).await;
    assert!(matches!(res, Err(Error::BadRequest(_))));
    assert_eq!(store.event_count(), 0);
}

#[tokio::test]
async fn get_event_returns_latest_and_drops_older() -> TbResult<()> {
    let (mut store, session) = setup();
    let mut first = activity_event();
    first.aspect_type = AspectType::Update;
    first.event_time = 50;
    store.strava_event_create(first).await?;
    store.strava_event_create(activity_event()).await?;
    let event = match get_event(session.user_id(), &mut store).await? {
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
    let (mut store, session) = setup();
    store.strava_event_create(activity_event()).await?;
    let res = get_event(session.user_id(), &mut store).await?;
    assert!(matches!(res, QueueRead::Limited { .. }));
    assert_eq!(store.event_count(), 1, "the event stays queued");
    reset_rate_limiter();
    Ok(())
}

#[tokio::test]
async fn get_event_not_rate_limited_returns_event() -> TbResult<()> {
    // When the rate limiter has budget, get_event dequeues normally.
    reset_rate_limiter();
    let (mut store, session) = setup();
    store.strava_event_create(activity_event()).await?;
    let res = get_event(session.user_id(), &mut store).await?;
    assert!(matches!(res, QueueRead::Event(ref e) if e.object_id == 10));
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
    queue("/activities/10", &activity_json(10, "Ride", None));
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
async fn process_try_again_keeps_event_queued() -> TbResult<()> {
    // A 429 (TryAgain) keeps the event queued: the rate-limit state is
    // updated from the response headers (in production), and the next
    // get_event probe will report Limited. No Stop event is created.
    reset_rate_limiter();
    let (mut store, mut session) = setup();
    store.strava_event_create(activity_event()).await?;
    queue_error("/activities/10", Error::TryAgain("rate limit"));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 1, "the event stays queued");
    Ok(())
}

#[tokio::test]
async fn process_activity_nontransient_drops_event() -> TbResult<()> {
    // A permanently bad activity (parse error, a 404's domain shape) is
    // dropped from the queue and logged — it is committed, so the event
    // is gone and the queue cannot wedge on it (spec #446 §4.6).
    let (mut store, mut session) = setup();
    store.strava_event_create(activity_event()).await?;
    queue_error(
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
    queue_error("/activities/10", Error::NotAuth(String::from("token dead")));
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
    queue_error(
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
    queue("/activities?after=0&per_page=25", "[]");
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
    queue(
        "/activities?after=0&per_page=25",
        &format!("[{}]", activity_json(10, "Ride", None)),
    );
    queue("/activities/10", &activity_json(10, "Ride", None));
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
    queue_error(
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
async fn process_sync_try_again_keeps_event_queued() -> TbResult<()> {
    reset_rate_limiter();
    let (mut store, mut session) = setup();
    store
        .strava_event_create(Event {
            object_type: ObjectType::Sync,
            owner_id: 42.into(),
            ..Default::default()
        })
        .await?;
    queue_error("/activities?after=0&per_page=25", Error::TryAgain("nope"));
    let summary = process(&mut session, &mut store).await?;
    assert_eq!(summary, Summary::default());
    assert_eq!(store.event_count(), 1, "the sync event stays queued");
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
    let mut session = strava_admin_session(UserId::from(1), 42.into());
    UserStore::user_create(&mut store.mem, "A", "User", &None).await?;
    UserStore::user_create(&mut store.mem, "B", "User", &None).await?;
    store.insert_user(strava_user(UserId::from(1), 42, true));
    store.insert_user(strava_user(UserId::from(2), 43, false));
    sync_users(&mut session, None, 100, false, &mut store).await?;
    assert_eq!(store.event_count(), 1);
    assert_eq!(store.events[0].owner_id, 42.into());
    Ok(())
}

#[tokio::test]
async fn sync_users_single() -> TbResult<()> {
    let (mut store, _) = setup();
    let mut session = strava_admin_session(UserId::from(1), 42.into());
    sync_users(&mut session, Some(UserId::from(1)), 100, false, &mut store).await?;
    assert_eq!(store.event_count(), 1);
    Ok(())
}

#[tokio::test]
async fn sync_users_unknown_user() {
    let (mut store, _) = setup();
    let mut session = strava_admin_session(UserId::from(1), 42.into());
    let res = sync_users(&mut session, Some(UserId::from(99)), 100, false, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}
