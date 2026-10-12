use tb_domain::{ActivityId, ActivityStore, Error, PartStore, TbResult, UserId};
use tb_strava::activity::{StravaActivity, delete_activity, strava_url, upsert_activity};
use tb_strava::test_support::{TestStravaSession, activity_json, gear_json, strava_user};
use tb_teststore::TestStore;

fn setup() -> (TestStore, TestStravaSession) {
    let mut store = TestStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let session = TestStravaSession::new(UserId::from(1), 42.into());
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
    let (mut store, mut session) = setup();
    let act = strava_activity(None)
        .into_activity(&mut session, &mut store)
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
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    let act = strava_activity(Some("b1"))
        .into_activity(&mut session, &mut store)
        .await?;
    let part = store.partid_get_part(act.gear.unwrap()).await?;
    assert_eq!(part.source.as_deref(), Some("b1"));
    assert_eq!(part.what, 1.into());
    assert_eq!(part.name, "Test Bike");
    assert_eq!(part.vendor, "Test Brand");
    assert_eq!(part.model, "Test Model");
    assert_eq!(session.requests, vec!["/gear/b1".to_string()]);
    Ok(())
}

#[tokio::test]
async fn into_activity_reuses_existing_gear() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    let first = strava_activity(Some("b1"))
        .into_activity(&mut session, &mut store)
        .await?;
    let second = strava_activity(Some("b1"))
        .into_activity(&mut session, &mut store)
        .await?;
    assert_eq!(first.gear, second.gear);
    let requests = session
        .requests
        .iter()
        .filter(|r| r.as_str() == "/gear/b1")
        .count();
    assert_eq!(requests, 1);
    Ok(())
}

#[tokio::test]
async fn upsert_activity_stores_new() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    let summary = upsert_activity(10, &mut session, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.get(&ActivityId::new(10)).is_some());
    let acts = store.get_all(&UserId::from(1)).await?;
    assert_eq!(acts.len(), 1);
    Ok(())
}

#[tokio::test]
async fn upsert_activity_updates_existing() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    upsert_activity(10, &mut session, &mut store).await?;
    let json = activity_json(10, "Ride", None).replace("25000.0", "30000.0");
    session.queue("/activities/10", &json);
    upsert_activity(10, &mut session, &mut store).await?;
    let acts = store.get_all(&UserId::from(1)).await?;
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].distance, Some(30000));
    Ok(())
}

#[tokio::test]
async fn upsert_activity_propagates_try_again() {
    let (mut store, mut session) = setup();
    session.queue_error("/activities/10", Error::TryAgain("rate limited"));
    let res = upsert_activity(10, &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::TryAgain(_))));
}

#[tokio::test]
async fn activity_strava_url() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    upsert_activity(10, &mut session, &mut store).await?;
    let url = strava_url(10, &session, &mut store).await?;
    assert_eq!(url, "https://strava.com/activities/10");
    Ok(())
}

#[tokio::test]
async fn activity_strava_url_missing() {
    let (mut store, session) = setup();
    let res = strava_url(99, &session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_activity_removes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    upsert_activity(10, &mut session, &mut store).await?;
    delete_activity(10, &session, &mut store).await?;
    let acts = store.get_all(&UserId::from(1)).await?;
    assert!(acts.is_empty());
    Ok(())
}

#[tokio::test]
async fn delete_activity_missing() {
    let (mut store, session) = setup();
    let res = delete_activity(42, &session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_activity_forbidden_for_other_user() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/activities/10", &activity_json(10, "Ride", None));
    upsert_activity(10, &mut session, &mut store).await?;
    let other = TestStravaSession::new(UserId::from(2), 43.into());
    let res = delete_activity(10, &other, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}
