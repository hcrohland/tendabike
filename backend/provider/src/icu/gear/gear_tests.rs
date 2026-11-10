use std::sync::{Arc, Mutex};

use super::super::test_support::{gear_json, icu_user, part, queue, requests, setup};
use super::*;

/// The `request_json` → `access_token` path reads the durable token from
/// `icu_users`; the import tests seed a tokened user for icu id `a1`.
fn expect_token(store: &mut super::super::traits::MockIcuDataStore) {
    store
        .expect_icuser_get_by_tbid()
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
}

/// A deserialized icu gear from the canned JSON (exercises the serde shape).
fn gear(id: &str, typ: &str) -> IcuGear {
    serde_json::from_str(&gear_json(id, typ)).unwrap()
}

#[tokio::test]
async fn into_partid_creates_bike() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // The part is not yet imported, so `into_partid` creates it. The facade
    // records the create's arguments (the icu logic's mapping of the gear to
    // part fields) and returns a part with a known id.
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
    let part_id = into_partid(&gear("b1", "Bike"), UserId::from(1), &facade, &mut store).await?;
    assert_eq!(part_id, Some(PartId::from(1)));
    let (what, name, vendor, model, source) = created
        .lock()
        .unwrap()
        .take()
        .expect("the icu logic created the part");
    assert_eq!(what, 1.into());
    assert_eq!(source, Some("icu:b1".to_string()));
    assert_eq!(name, "Test Gear");
    assert_eq!(vendor, "");
    assert_eq!(model, "");
    Ok(())
}

#[tokio::test]
async fn into_partid_creates_trainer() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // A `Trainer` maps to the same part type as a bike (what 1).
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
    let part_id = into_partid(&gear("t1", "Trainer"), UserId::from(1), &facade, &mut store).await?;
    assert_eq!(part_id, Some(PartId::from(1)));
    let (what, _name, _vendor, _model, source) = created
        .lock()
        .unwrap()
        .take()
        .expect("the icu logic created the part");
    assert_eq!(what, 1.into());
    assert_eq!(source, Some("icu:t1".to_string()));
    Ok(())
}

#[tokio::test]
async fn into_partid_creates_shoes() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // A `Shoes` gear maps to what 301; vendor and model are left empty.
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
    let part_id = into_partid(&gear("s1", "Shoes"), UserId::from(1), &facade, &mut store).await?;
    assert_eq!(part_id, Some(PartId::from(1)));
    let (what, _name, vendor, model, source) = created
        .lock()
        .unwrap()
        .take()
        .expect("the icu logic created the part");
    assert_eq!(what, 301.into());
    assert_eq!(source, Some("icu:s1".to_string()));
    assert_eq!(vendor, "");
    assert_eq!(model, "");
    Ok(())
}

#[tokio::test]
async fn into_partid_skips_unsupported() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // An unsupported type (a helmet) is skipped: `into_partid` returns `None`
    // and the create never fires (no `expect_create_part` is set, so an
    // unexpected create would panic the mock).
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    let part_id = into_partid(&gear("h1", "Helmet"), UserId::from(1), &facade, &mut store).await?;
    assert_eq!(part_id, None);
    Ok(())
}

#[tokio::test]
async fn into_partid_reuses_existing() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // The part already exists (the namespaced source is found): `into_partid`
    // reuses it and the create never fires (no `expect_create_part` is set, so
    // an unexpected create would panic the mock).
    let bike = part(
        PartId::from(7),
        UserId::from(1),
        1.into(),
        Some("icu:b1".into()),
    );
    let bike_rec = bike.clone();
    facade
        .expect_part_by_source()
        .returning(move |_s, _store| Ok(Some(bike_rec.clone())));
    let part_id = into_partid(&gear("b1", "Bike"), UserId::from(1), &facade, &mut store).await?;
    assert_eq!(part_id, Some(PartId::from(7)));
    Ok(())
}

#[tokio::test]
async fn update_gear_imports_bikes_and_shoes() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    expect_token(&mut store);
    // The gear list: a bike, a trainer, shoes, and an unsupported helmet.
    queue(
        "/athlete/a1/gear",
        r#"[
            {"id":"b1","name":"Road Bike","type":"Bike","distance":1000.0,"retired":false},
            {"id":"t1","name":"Trainer","type":"Trainer","distance":500.0,"retired":false},
            {"id":"s1","name":"Shoes","type":"Shoes","distance":800.0,"retired":false},
            {"id":"h1","name":"Helmet","type":"Helmet","distance":0.0,"retired":false}
        ]"#,
    );
    // No gear is imported yet: each supported gear creates a part. The facade
    // assigns ids in order and records each part's `what` (bike/trainer vs
    // shoes); the helmet is skipped.
    let next_id = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let next_id_rec = next_id.clone();
    let whats = Arc::new(Mutex::new(Vec::new()));
    let whats_rec = whats.clone();
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    facade.expect_create_part().returning(
        move |_name, _vendor, _model, what, _source, _purchase, _user, _store| {
            whats_rec.lock().unwrap().push(what);
            let id = next_id_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            Ok(part(PartId::from(id), UserId::from(1), what, None))
        },
    );
    let parts = update_gear(UserId::from(1), &facade, &mut store).await?;
    assert_eq!(
        parts,
        vec![PartId::from(1), PartId::from(2), PartId::from(3)]
    );
    assert_eq!(
        whats.lock().unwrap().as_slice(),
        [1.into(), 1.into(), 301.into()],
        "the bike and trainer are what 1, the shoes are what 301"
    );
    assert_eq!(requests(), vec!["/athlete/a1/gear".to_string()]);
    Ok(())
}
