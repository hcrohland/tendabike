use std::sync::{Arc, Mutex};

use super::super::test_support::{gear_json, part, setup};
use super::*;
use crate::facade::{Error, Part};
use crate::test_support::{queue, queue_error, requests};

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

/// A part carrying `source`, owned by `owner` — the source's first letter
/// picks the URL shape.
fn sourced_part(source: &str, owner: UserId) -> Part {
    part(PartId::from(1), owner, 1.into(), Some(source.to_string()))
}

#[test]
fn gear_strava_url_bike() -> TbResult<()> {
    let url = super::super::driver::StravaDriver::gear_url_from_part(&sourced_part(
        "b42",
        UserId::from(1),
    ))?;
    assert_eq!(url, Some("https://strava.com/bikes/42".to_string()));
    Ok(())
}

#[test]
fn gear_strava_url_shoes() -> TbResult<()> {
    let url = super::super::driver::StravaDriver::gear_url_from_part(&sourced_part(
        "g1",
        UserId::from(1),
    ))?;
    assert_eq!(
        url,
        Some("https://www.strava.com/settings/gear".to_string())
    );
    Ok(())
}

#[test]
fn gear_strava_url_unknown_source() -> TbResult<()> {
    let res = super::super::driver::StravaDriver::gear_url_from_part(&sourced_part(
        "x9",
        UserId::from(1),
    ));
    assert!(matches!(res, Err(Error::NotFound(_))));
    Ok(())
}

#[test]
fn gear_strava_url_missing_source() -> TbResult<()> {
    // A locally created part (no Strava source): `strava_url` is the typed
    // absence (`None`) — the route maps it to a no-op (ADR-0014).
    let res = super::super::driver::StravaDriver::gear_url_from_part(&part(
        PartId::from(1),
        UserId::from(1),
        1.into(),
        None,
    ))?;
    assert_eq!(res, None);
    Ok(())
}
