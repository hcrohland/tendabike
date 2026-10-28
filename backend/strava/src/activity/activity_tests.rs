use std::sync::{Arc, Mutex};

use super::*;
use crate::test_support::{
    MockStravaConn, activity, activity_json, gear_json, part, queue, queue_error, requests,
    strava_session,
};
use tb_domain::MockSession;

fn setup() -> (MockStravaConn, MockSession) {
    let store = MockStravaConn::new();
    let session = strava_session(UserId::from(1), 42.into());
    (store, session)
}

fn strava_activity(gear: Option<&str>) -> StravaActivity {
    serde_json::from_str(&activity_json(10, "Ride", gear)).unwrap()
}

#[test]
fn get_type_maps_strava_types() {
    assert_eq!(StravaActivity::get_type("Ride").unwrap(), 1.into());
    assert_eq!(StravaActivity::get_type("VirtualRide").unwrap(), 5.into());
    assert_eq!(StravaActivity::get_type("EBikeRide").unwrap(), 9.into());
    assert_eq!(StravaActivity::get_type("Run").unwrap(), 3.into());
    assert_eq!(StravaActivity::get_type("VirtualRun").unwrap(), 3.into());
    assert_eq!(StravaActivity::get_type("Workout").unwrap(), 0.into());
    assert_eq!(StravaActivity::get_type("Golf").unwrap(), 0.into());
}

#[test]
fn get_type_rejects_unknown() {
    assert!(matches!(
        StravaActivity::get_type("Frobnicate"),
        Err(Error::BadRequest(_))
    ));
}

#[tokio::test]
async fn into_activity_maps_fields() -> TbResult<()> {
    // No gear: `into_activity` is a pure field mapping (no store read), so the
    // canned store is unused.
    let (mut _store, mut session) = setup();
    let act = strava_activity(None)
        .into_activity(&mut session, &mut _store)
        .await?;
    assert_eq!(act.id, ActivityId::new(10));
    assert_eq!(act.what, 1.into());
    assert_eq!(act.user_id, UserId::from(1));
    assert!(act.gear.is_none());
    assert_eq!(act.name, "Test Ride");
    assert_eq!(act.start.unix_timestamp(), 1767348000);
    assert_eq!(act.duration, 3600);
    assert_eq!(act.time, Some(3300));
    assert_eq!(act.distance, Some(25000));
    assert_eq!(act.climb, Some(300));
    assert!(act.descend.is_none());
    assert_eq!(act.energy, Some(4000));
    assert_eq!(act.device_name.as_deref(), Some("Test Device"));
    assert!(act.external_id.is_none());
    Ok(())
}

#[tokio::test]
async fn into_activity_creates_missing_gear() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // The gear is not imported: the Strava gear fetch runs and the part is
    // created. The canned store records the create's arguments and returns a
    // part with a known id; the activity's `gear` is that id.
    let created = Arc::new(Mutex::new(None));
    let created_rec = created.clone();
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    store.expect_part_create().returning(
        move |what, name, vendor, model, _p, source, _u, _o, _s| {
            created_rec
                .lock()
                .unwrap()
                .replace((what, name, vendor, model, source));
            Ok(part(PartId::from(1), UserId::from(1), 1.into(), None))
        },
    );
    let act = strava_activity(Some("b1"))
        .into_activity(&mut session, &mut store)
        .await?;
    assert_eq!(act.gear, Some(PartId::from(1)));
    let (what, name, vendor, model, source) = created
        .lock()
        .unwrap()
        .take()
        .expect("the gear part was created");
    assert_eq!(what, 1.into());
    assert_eq!(source, Some("b1".to_string()));
    assert_eq!(name, "Test Bike");
    assert_eq!(vendor, "Test Brand");
    assert_eq!(model, "Test Model");
    assert_eq!(requests(), vec!["/gear/b1".to_string()]);
    Ok(())
}

#[tokio::test]
async fn into_activity_reuses_existing_gear() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // First activity: no part yet (the fetch runs, the create fires). Second
    // activity: the part exists, so `by_source` returns it and no second
    // Strava fetch happens.
    let seen = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let seen_rec = seen.clone();
    let bike = part(
        PartId::from(1),
        UserId::from(1),
        1.into(),
        Some("b1".into()),
    );
    let bike_source = bike.clone();
    let bike_create = bike.clone();
    let bike_get = bike.clone();
    store.expect_partid_get_by_source().returning(move |_s| {
        let n = seen_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n < 2 {
            Ok(None)
        } else {
            Ok(Some(bike_source.id))
        }
    });
    store.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _p, _source, _u, _o, _s| Ok(bike_create.clone()),
    );
    store
        .expect_partid_get_part()
        .returning(move |_pid| Ok(bike_get.clone()));
    let first = strava_activity(Some("b1"))
        .into_activity(&mut session, &mut store)
        .await?;
    let second = strava_activity(Some("b1"))
        .into_activity(&mut session, &mut store)
        .await?;
    assert_eq!(first.gear, second.gear);
    let requests = requests()
        .iter()
        .filter(|r| r.as_str() == "/gear/b1")
        .count();
    assert_eq!(requests, 1);
    Ok(())
}

#[tokio::test]
async fn upsert_activity_stores_new() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // A new activity: the read is `None`, so the create fires (gear `None`
    // keeps the register's usage/attachment reads out of the picture).
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    let summary = upsert_activity(10, &mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.get(&ActivityId::new(10)).is_some());
    Ok(())
}

#[tokio::test]
async fn upsert_activity_updates_existing() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The read is `None` for the first upsert (create) and `Some` for the
    // second (update): one canned read with a call counter drives both.
    let seen = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let seen_rec = seen.clone();
    store.expect_activity_read_by_id().returning(move |_aid| {
        let n = seen_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(activity(ActivityId::new(10), UserId::from(1))))
        }
    });
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    store
        .expect_activity_update()
        .times(1)
        .returning(|_act| Ok(activity(ActivityId::new(10), UserId::from(1))));
    upsert_activity(10, &mut session, &mut store).await?;
    let json = activity_json(10, "Ride", None).replace("25000.0", "30000.0");
    queue("/activities/10", &json);
    let summary = upsert_activity(10, &mut session, &mut store).await?;
    assert!(
        summary.activities.get(&ActivityId::new(10)).is_some(),
        "the updated activity is in the summary"
    );
    Ok(())
}

#[tokio::test]
async fn upsert_activity_propagates_try_again() {
    let (mut _store, mut session) = setup();
    // The Strava fetch 429s: the `TryAgain` is surfaced (no store read fires).
    queue_error("/activities/10", Error::TryAgain("rate limited"));
    let res = upsert_activity(10, &mut session, &mut _store).await;
    assert!(matches!(res, Err(Error::TryAgain(_))));
}

#[tokio::test]
async fn activity_strava_url() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The upsert creates the activity (read `None`, create), then `strava_url`
    // reads it back (read `Some`).
    let seen = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let seen_rec = seen.clone();
    store.expect_activity_read_by_id().returning(move |_aid| {
        let n = seen_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(activity(ActivityId::new(10), UserId::from(1))))
        }
    });
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    upsert_activity(10, &mut session, &mut store).await?;
    let url = strava_url(10, &session, &mut store).await?;
    assert_eq!(url, "https://strava.com/activities/10");
    Ok(())
}

#[tokio::test]
async fn activity_strava_url_missing() {
    let (mut store, session) = setup();
    // The activity does not exist: the read is `None`, so `strava_url` is
    // `NotFound`.
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    let res = strava_url(99, &session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_activity_removes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The upsert creates the activity (read `None`, create); the delete reads
    // it back (read `Some`), then deletes it. The delete reports the activity
    // as a tombstone.
    let seen = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let seen_rec = seen.clone();
    store.expect_activity_read_by_id().returning(move |_aid| {
        let n = seen_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(activity(ActivityId::new(10), UserId::from(1))))
        }
    });
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    store.expect_activity_delete().returning(|_aid| Ok(1));
    upsert_activity(10, &mut session, &mut store).await?;
    let summary = delete_activity(10, &session, &mut store).await?;
    assert!(
        summary.activities.is_tombstoned(&ActivityId::new(10)),
        "the deleted activity is reported as a tombstone"
    );
    Ok(())
}

#[tokio::test]
async fn delete_activity_missing() {
    let (mut store, session) = setup();
    // The activity does not exist: the read is `None`, so the delete is
    // `NotFound`.
    store
        .expect_activity_read_by_id()
        .returning(|_aid| Ok(None));
    let res = delete_activity(42, &session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_activity_forbidden_for_other_user() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The upsert creates the activity (owned by user 1); the delete from user
    // 2 is `Forbidden` by the ownership check on the read.
    let seen = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let seen_rec = seen.clone();
    store.expect_activity_read_by_id().returning(move |_aid| {
        let n = seen_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(activity(ActivityId::new(10), UserId::from(1))))
        }
    });
    store
        .expect_activity_create()
        .returning(move |act| Ok(activity(act.id, act.user_id)));
    let _ = upsert_activity(10, &mut session, &mut store).await?;
    let other = strava_session(UserId::from(2), 43.into());
    let res = delete_activity(10, &other, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}
