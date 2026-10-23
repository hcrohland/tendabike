use super::*;
use crate::test_support::{TestStravaSession, TestStravaStore, gear_json, strava_user};

fn setup() -> (TestStravaStore, TestStravaSession) {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let session = TestStravaSession::new(UserId::from(1), 42.into());
    (store, session)
}

#[tokio::test]
async fn into_partid_creates_bike() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    let part_id = into_partid("b1".to_string(), &mut session, &mut store).await?;
    let part = PartStore::partid_get_part(&mut store.mem, part_id).await?;
    assert_eq!(part.what, 1.into());
    assert_eq!(part.source.as_deref(), Some("b1"));
    assert_eq!(part.vendor, "Test Brand");
    assert_eq!(part.model, "Test Model");
    assert_eq!(part.name, "Test Bike");
    Ok(())
}

#[tokio::test]
async fn into_partid_creates_shoes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue(
        "/gear/g1",
        r#"{"id":"g1","name":"Test Shoes","brand_name":null,"model_name":null,"frame_type":null}"#,
    );
    let part_id = into_partid("g1".to_string(), &mut session, &mut store).await?;
    let part = PartStore::partid_get_part(&mut store.mem, part_id).await?;
    assert_eq!(part.what, 301.into());
    assert_eq!(part.vendor, "");
    assert_eq!(part.model, "");
    Ok(())
}

#[tokio::test]
async fn into_partid_reuses_existing() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    let first = into_partid("b1".to_string(), &mut session, &mut store).await?;
    let second = into_partid("b1".to_string(), &mut session, &mut store).await?;
    assert_eq!(first, second);
    assert_eq!(session.requests(), vec!["/gear/b1".to_string()]);
    Ok(())
}

#[tokio::test]
async fn into_partid_propagates_error() {
    let (mut store, mut session) = setup();
    session.queue_error("/gear/b1", Error::BadRequest("nope".into()));
    let res = into_partid("b1".to_string(), &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::AnyFailure(_))));
}

async fn seed_part(
    store: &mut TestStravaStore,
    session: &mut TestStravaSession,
    source: &str,
) -> TbResult<PartId> {
    session.queue(&format!("/gear/{source}"), &gear_json(source, Some(0)));
    into_partid(source.to_string(), session, store).await
}

#[tokio::test]
async fn gear_strava_url_bike() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "b42").await?;
    let url = strava_url(i32::from(part_id), &mut session, &mut store).await?;
    assert_eq!(url, "https://strava.com/bikes/42");
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_shoes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "g1").await?;
    let url = strava_url(i32::from(part_id), &mut session, &mut store).await?;
    assert_eq!(url, "https://www.strava.com/settings/gear");
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_unknown_source() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "x9").await?;
    let res = strava_url(i32::from(part_id), &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_missing_source() {
    let (mut store, mut session) = setup();
    let part = Part::create(
        "Local Part".into(),
        "Brand".into(),
        "Model".into(),
        1.into(),
        None,
        time::OffsetDateTime::now_utc(),
        &session,
        &mut store,
    )
    .await
    .unwrap();
    let res = strava_url(i32::from(part.id), &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn gear_strava_url_forbidden_other_user() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "b42").await?;
    let mut other = TestStravaSession::new(UserId::from(2), 43.into());
    let res = strava_url(i32::from(part_id), &mut other, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}
