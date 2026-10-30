use std::sync::{Arc, Mutex};

use super::*;
use crate::test_support::{
    MockStravaConn, gear_json, part, queue, queue_error, requests, strava_session,
};
use tb_domain::MockSession;

fn setup() -> (MockStravaConn, MockSession) {
    let store = MockStravaConn::new();
    let session = strava_session(UserId::from(1), 42.into());
    (store, session)
}

#[tokio::test]
async fn into_partid_creates_bike() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // The part is not yet imported (before and after the Strava gear fetch),
    // so `into_partid` creates it. The canned store records the create's
    // arguments (the strava logic's mapping of the gear JSON to part fields)
    // and returns a part with a known id.
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
    let part_id = into_partid("b1".to_string(), &mut session, &mut store).await?;
    assert_eq!(part_id, PartId::from(1));
    let (what, name, vendor, model, source) = created
        .lock()
        .unwrap()
        .take()
        .expect("the strava logic created the part");
    assert_eq!(what, 1.into());
    assert_eq!(source, Some("b1".to_string()));
    assert_eq!(name, "Test Bike");
    assert_eq!(vendor, "Test Brand");
    assert_eq!(model, "Test Model");
    assert_eq!(requests(), vec!["/gear/b1".to_string()]);
    Ok(())
}

#[tokio::test]
async fn into_partid_creates_shoes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue(
        "/gear/g1",
        r#"{"id":"g1","name":"Test Shoes","brand_name":null,"model_name":null,"frame_type":null}"#,
    );
    // `frame_type` null maps to shoes (what 301); a missing brand/model maps
    // to the empty string. The canned store records the create's arguments.
    let created = Arc::new(Mutex::new(None));
    let created_rec = created.clone();
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    store.expect_part_create().returning(
        move |what, name, vendor, model, _p, source, _u, _o, _s| {
            created_rec
                .lock()
                .unwrap()
                .replace((what, name, vendor, model, source));
            Ok(part(PartId::from(1), UserId::from(1), 301.into(), None))
        },
    );
    let part_id = into_partid("g1".to_string(), &mut session, &mut store).await?;
    assert_eq!(part_id, PartId::from(1));
    let (what, name, vendor, model, source) = created
        .lock()
        .unwrap()
        .take()
        .expect("the strava logic created the part");
    assert_eq!(what, 301.into());
    assert_eq!(source, Some("g1".to_string()));
    assert_eq!(name, "Test Shoes");
    assert_eq!(vendor, "");
    assert_eq!(model, "");
    Ok(())
}

#[tokio::test]
async fn into_partid_reuses_existing() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // First `into_partid`: no part yet (the Strava fetch runs, the create
    // fires). Second `into_partid`: the part now exists, so `by_source`
    // returns it and no second Strava fetch happens.
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
    let first = into_partid("b1".to_string(), &mut session, &mut store).await?;
    let second = into_partid("b1".to_string(), &mut session, &mut store).await?;
    assert_eq!(first, second);
    assert_eq!(requests(), vec!["/gear/b1".to_string()]);
    Ok(())
}

#[tokio::test]
async fn into_partid_propagates_error() {
    let (mut store, mut session) = setup();
    // The part is not imported and the Strava gear fetch fails: the error is
    // wrapped (`context("Couldn't map gear")`) and surfaced as `AnyFailure`.
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    queue_error("/gear/b1", Error::BadRequest("nope".into()));
    let res = into_partid("b1".to_string(), &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::AnyFailure(_))));
}

async fn seed_part(
    store: &mut MockStravaConn,
    session: &mut MockSession,
    source: &str,
) -> TbResult<PartId> {
    // A part carrying `source` is created: no existing part for the source,
    // the Strava gear fetch, then the create.
    queue(&format!("/gear/{source}"), &gear_json(source, Some(0)));
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    let src = source.to_string();
    store.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _p, _source, _u, _o, _s| {
            Ok(part(
                PartId::from(1),
                UserId::from(1),
                1.into(),
                Some(src.clone()),
            ))
        },
    );
    into_partid(source.to_string(), session, store).await
}

/// A part carrying `source`, owned by `owner` — the `partid_get_part` read
/// `strava_url` drives (the source's first letter picks the URL shape).
fn sourced_part(source: &str, owner: UserId) -> Part {
    part(PartId::from(1), owner, 1.into(), Some(source.to_string()))
}

#[tokio::test]
async fn gear_strava_url_bike() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "b42").await?;
    store
        .expect_partid_get_part()
        .returning(move |_pid| Ok(sourced_part("b42", UserId::from(1))));
    let url = strava_url(i32::from(part_id), &mut session, &mut store).await?;
    assert_eq!(url, "https://strava.com/bikes/42");
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_shoes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "g1").await?;
    store
        .expect_partid_get_part()
        .returning(move |_pid| Ok(sourced_part("g1", UserId::from(1))));
    let url = strava_url(i32::from(part_id), &mut session, &mut store).await?;
    assert_eq!(url, "https://www.strava.com/settings/gear");
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_unknown_source() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "x9").await?;
    store
        .expect_partid_get_part()
        .returning(move |_pid| Ok(sourced_part("x9", UserId::from(1))));
    let res = strava_url(i32::from(part_id), &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_missing_source() {
    let (mut store, mut session) = setup();
    // A locally created part (no Strava source): `strava_url` is `NotFound`.
    store.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _p, _source, _u, _o, _s| {
            Ok(part(PartId::from(1), UserId::from(1), 1.into(), None))
        },
    );
    let created = Part::create(
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
    store
        .expect_partid_get_part()
        .returning(move |_pid| Ok(part(PartId::from(1), UserId::from(1), 1.into(), None)));
    let res = strava_url(i32::from(created.id), &mut session, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn gear_strava_url_forbidden_other_user() -> TbResult<()> {
    let (mut store, mut session) = setup();
    let part_id = seed_part(&mut store, &mut session, "b42").await?;
    // The part is owned by user 1; a request from user 2 (no shop) is
    // `Forbidden` by the ownership check.
    store
        .expect_partid_get_part()
        .returning(move |_pid| Ok(sourced_part("b42", UserId::from(1))));
    let mut other = strava_session(UserId::from(2), 43.into());
    let res = strava_url(i32::from(part_id), &mut other, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}
