use std::sync::{Arc, Mutex};

use super::super::test_support::{
    gear_json, part, queue, queue_error, requests, setup, strava_session,
};
use super::*;
use crate::MockEntityFacade;

#[tokio::test]
async fn into_partid_creates_bike() -> TbResult<()> {
    let (mut store, mut facade, _session) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // The part is not yet imported (before and after the Strava gear fetch),
    // so `into_partid` creates it. The facade records the create's
    // arguments (the strava logic's mapping of the gear JSON to part fields)
    // and returns a part with a known id.
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
    let part_id = into_partid("b1".to_string(), UserId::from(1), &facade, &mut store).await?;
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
    let (mut store, mut facade, _session) = setup();
    queue(
        "/gear/g1",
        r#"{"id":"g1","name":"Test Shoes","brand_name":null,"model_name":null,"frame_type":null}"#,
    );
    // `frame_type` null maps to shoes (what 301); a missing brand/model maps
    // to the empty string. The facade records the create's arguments.
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
            Ok(part(PartId::from(1), UserId::from(1), 301.into(), None))
        },
    );
    let part_id = into_partid("g1".to_string(), UserId::from(1), &facade, &mut store).await?;
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
    let (mut store, mut facade, _session) = setup();
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
    let first = into_partid("b1".to_string(), UserId::from(1), &facade, &mut store).await?;
    let second = into_partid("b1".to_string(), UserId::from(1), &facade, &mut store).await?;
    assert_eq!(first, second);
    assert_eq!(requests(), vec!["/gear/b1".to_string()]);
    Ok(())
}

#[tokio::test]
async fn into_partid_propagates_error() {
    let (mut store, mut facade, _session) = setup();
    // The part is not imported and the Strava gear fetch fails: the error is
    // wrapped (`context("Couldn't map gear")`) and surfaced as `AnyFailure`.
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    queue_error("/gear/b1", Error::BadRequest("nope".into()));
    let res = into_partid("b1".to_string(), UserId::from(1), &facade, &mut store).await;
    assert!(matches!(res, Err(Error::AnyFailure(_))));
}

async fn seed_part(
    store: &mut MockStravaDataStore,
    facade: &mut MockEntityFacade<MockStravaDataStore>,
    source: &str,
) -> TbResult<PartId> {
    // A part carrying `source` is created: no existing part for the source,
    // the Strava gear fetch, then the create.
    queue(&format!("/gear/{source}"), &gear_json(source, Some(0)));
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    let src = source.to_string();
    facade.expect_create_part().returning(
        move |_name, _vendor, _model, _what, _source, _purchase, _user, _store| {
            Ok(part(
                PartId::from(1),
                UserId::from(1),
                1.into(),
                Some(src.clone()),
            ))
        },
    );
    into_partid(source.to_string(), UserId::from(1), facade, store).await
}

/// A part carrying `source`, owned by `owner` — the `part` read
/// `strava_url` drives (the source's first letter picks the URL shape).
fn sourced_part(source: &str, owner: UserId) -> Part {
    part(PartId::from(1), owner, 1.into(), Some(source.to_string()))
}

#[tokio::test]
async fn gear_strava_url_bike() -> TbResult<()> {
    let (mut store, mut facade, session) = setup();
    let part_id = seed_part(&mut store, &mut facade, "b42").await?;
    facade
        .expect_part()
        .returning(move |_pid, _user, _store| Ok(sourced_part("b42", UserId::from(1))));
    let url = strava_url(i32::from(part_id), Box::new(session), &facade, &mut store).await?;
    assert_eq!(url, Some("https://strava.com/bikes/42".to_string()));
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_shoes() -> TbResult<()> {
    let (mut store, mut facade, session) = setup();
    let part_id = seed_part(&mut store, &mut facade, "g1").await?;
    facade
        .expect_part()
        .returning(move |_pid, _user, _store| Ok(sourced_part("g1", UserId::from(1))));
    let url = strava_url(i32::from(part_id), Box::new(session), &facade, &mut store).await?;
    assert_eq!(
        url,
        Some("https://www.strava.com/settings/gear".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_unknown_source() -> TbResult<()> {
    let (mut store, mut facade, session) = setup();
    let part_id = seed_part(&mut store, &mut facade, "x9").await?;
    facade
        .expect_part()
        .returning(move |_pid, _user, _store| Ok(sourced_part("x9", UserId::from(1))));
    let res = strava_url(i32::from(part_id), Box::new(session), &facade, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_missing_source() -> TbResult<()> {
    let (mut store, mut facade, session) = setup();
    // A locally created part (no Strava source): `strava_url` is the typed
    // absence (`None`) — the route maps it to a no-op (ADR-0014).
    facade.expect_part().returning(move |_pid, _user, _store| {
        Ok(part(PartId::from(1), UserId::from(1), 1.into(), None))
    });
    let res = strava_url(1, Box::new(session), &facade, &mut store).await?;
    assert_eq!(res, None);
    Ok(())
}

#[tokio::test]
async fn gear_strava_url_forbidden_other_user() -> TbResult<()> {
    let (mut store, mut facade, _session) = setup();
    let part_id = seed_part(&mut store, &mut facade, "b42").await?;
    // The part is owned by user 1; a request from user 2 (no shop) is
    // `Forbidden` by the ownership check.
    facade
        .expect_part()
        .returning(move |_pid, _user, _store| Err(Error::Forbidden("not your part".to_string())));
    let other = strava_session(UserId::from(2), 43.into());
    let res = strava_url(i32::from(part_id), Box::new(other), &facade, &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
    Ok(())
}
