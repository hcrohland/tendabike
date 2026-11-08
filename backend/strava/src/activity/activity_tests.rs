use std::sync::{Arc, Mutex};

use super::*;
use crate::test_support::{
    MockStravaDataStore, activity, activity_json, gear_json, part, queue, queue_error, requests,
    strava_session,
};
use tb_domain::MockSession;
use tb_provider::MockDomainFacade;

fn setup() -> (
    MockStravaDataStore,
    MockDomainFacade<MockStravaDataStore>,
    MockSession,
) {
    let store = MockStravaDataStore::new();
    let facade = MockDomainFacade::<MockStravaDataStore>::new();
    let session = strava_session(UserId::from(1), 42.into());
    (store, facade, session)
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

/// activity_id renders the prefixed form "strava:<n>"
#[test]
fn activity_id_renders_prefixed() {
    assert_eq!(format!("{}", activity_id(12345)), "strava:12345");
}

/// numeric_id recovers the raw numeric id from a strava id
#[test]
fn numeric_id_recovers_raw() {
    assert_eq!(numeric_id(&activity_id(12345)), Some(12345));
}

/// numeric_id is None for a non-strava id
#[test]
fn numeric_id_none_for_non_strava() {
    assert_eq!(numeric_id(&ActivityId::from("icu:i55751783")), None);
}

#[tokio::test]
async fn into_activity_maps_fields() -> TbResult<()> {
    // No gear: `into_activity` is a pure field mapping (no store read), so the
    // canned store and facade are unused.
    let (mut _store, _facade, _) = setup();
    let act = strava_activity(None)
        .into_activity(UserId::from(1), &_facade, &mut _store)
        .await?;
    assert_eq!(act.id, activity_id(10));
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
    let (mut store, mut facade, _) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // The gear is not imported: the Strava gear fetch runs and the part is
    // created. The facade records the create's arguments and returns a
    // part with a known id; the activity's `gear` is that id.
    let created = Arc::new(Mutex::new(None));
    let created_rec = created.clone();
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    facade.expect_create_part().returning(
        move |name, vendor, model, what, source, _p, _u, _store| {
            created_rec
                .lock()
                .unwrap()
                .replace((what, name, vendor, model, source));
            Ok(part(PartId::from(1), UserId::from(1), 1.into(), None))
        },
    );
    let act = strava_activity(Some("b1"))
        .into_activity(UserId::from(1), &facade, &mut store)
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
    let (mut store, mut facade, _) = setup();
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
    facade.expect_part_by_source().returning(move |_s, _store| {
        let n = seen_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n < 2 {
            Ok(None)
        } else {
            Ok(Some(bike_source.clone()))
        }
    });
    facade
        .expect_create_part()
        .returning(move |_n, _v, _m, _w, _s, _p, _u, _store| Ok(bike_create.clone()));
    let first = strava_activity(Some("b1"))
        .into_activity(UserId::from(1), &facade, &mut store)
        .await?;
    let second = strava_activity(Some("b1"))
        .into_activity(UserId::from(1), &facade, &mut store)
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
    let (mut store, mut facade, _) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // A new activity: the facade's upsert returns a summary with the activity.
    facade
        .expect_upsert_activity()
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    let summary = upsert_activity(10, UserId::from(1), &facade, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.get(&activity_id(10)).is_some());
    Ok(())
}

#[tokio::test]
async fn upsert_activity_updates_existing() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    queue("/activities/10", &activity_json(10, "Ride", None));
    // The facade's upsert returns a summary with the activity (both calls).
    facade
        .expect_upsert_activity()
        .times(2)
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    upsert_activity(10, UserId::from(1), &facade, &mut store).await?;
    let json = activity_json(10, "Ride", None).replace("25000.0", "30000.0");
    queue("/activities/10", &json);
    let summary = upsert_activity(10, UserId::from(1), &facade, &mut store).await?;
    assert!(
        summary.activities.get(&activity_id(10)).is_some(),
        "the updated activity is in the summary"
    );
    Ok(())
}

#[tokio::test]
async fn upsert_activity_propagates_try_again() {
    let (mut _store, _facade, _) = setup();
    // The Strava fetch 429s: the `TryAgain` is surfaced (no store read fires).
    queue_error("/activities/10", Error::TryAgain("rate limited"));
    let res = upsert_activity(10, UserId::from(1), &_facade, &mut _store).await;
    assert!(matches!(res, Err(Error::TryAgain(_))));
}

#[tokio::test]
async fn activity_strava_url() -> TbResult<()> {
    let (mut store, mut facade, session) = setup();
    // The facade's read returns the activity (the ownership check passes).
    facade
        .expect_read_activity()
        .returning(move |_id, _user, _store| Ok(activity(activity_id(10), UserId::from(1))));
    let url = strava_url(activity_id(10), Box::new(session), &facade, &mut store).await?;
    assert_eq!(url, "https://strava.com/activities/10");
    Ok(())
}

#[tokio::test]
async fn activity_strava_url_missing() {
    let (mut store, mut facade, session) = setup();
    // The activity does not exist: the facade's read is `NotFound`.
    facade
        .expect_read_activity()
        .returning(|_id, _user, _store| Err(Error::NotFound("".to_string())));
    let res = strava_url(activity_id(99), Box::new(session), &facade, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_activity_removes() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    // The facade's delete returns a summary with the activity tombstoned.
    facade
        .expect_delete_activity()
        .returning(move |id, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.tombstone(id);
            Ok(summary)
        });
    let summary = delete_activity(10, UserId::from(1), &facade, &mut store).await?;
    assert!(
        summary.activities.is_tombstoned(&activity_id(10)),
        "the deleted activity is reported as a tombstone"
    );
    Ok(())
}

#[tokio::test]
async fn delete_activity_missing() {
    let (mut store, mut facade, _) = setup();
    // The activity does not exist: the facade's delete is `NotFound`.
    facade
        .expect_delete_activity()
        .returning(|_id, _user, _store| Err(Error::NotFound("".to_string())));
    let res = delete_activity(42, UserId::from(1), &facade, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_activity_forbidden_for_other_user() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    // The delete from user 2 is `Forbidden` by the ownership check.
    facade
        .expect_delete_activity()
        .returning(|_id, _user, _store| Err(Error::Forbidden("not your activity".to_string())));
    let res = delete_activity(10, UserId::from(2), &facade, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}
