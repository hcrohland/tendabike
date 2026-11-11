use super::super::test_support::{
    activity_json, icu_user, part, queue, queue_error, requests, setup, stub_activity_json,
};
use super::*;
use tb_domain::{PartId, Summary};

/// The `request_json` → `access_token` path reads the durable token from
/// `icu_users`; the import tests seed a tokened user.
fn expect_token(store: &mut super::super::traits::MockIcuDataStore) {
    store
        .expect_icuser_get_by_tbid()
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
}

fn icu_activity(gear: Option<&str>) -> IcuActivity {
    serde_json::from_str(&activity_json("10", "Ride", gear)).unwrap()
}

// ─── get_type ───────────────────────────────────────────────────────────────

#[test]
fn get_type_maps_cycling_subtypes() {
    assert_eq!(IcuActivity::get_type("Ride").unwrap(), 1.into());
    assert_eq!(
        IcuActivity::get_type("MountainBikeRide").unwrap(),
        11.into()
    );
    assert_eq!(
        IcuActivity::get_type("EMountainBikeRide").unwrap(),
        11.into()
    );
    assert_eq!(IcuActivity::get_type("GravelRide").unwrap(), 12.into());
    assert_eq!(IcuActivity::get_type("Cyclocross").unwrap(), 13.into());
    assert_eq!(IcuActivity::get_type("EBikeRide").unwrap(), 9.into());
    assert_eq!(IcuActivity::get_type("VirtualRide").unwrap(), 5.into());
    // The cycling-subtype fallbacks.
    for t in [
        "Handcycle",
        "TrackRide",
        "Velomobile",
        "NordicSki",
        "RollerSki",
    ] {
        assert_eq!(IcuActivity::get_type(t).unwrap(), 0.into(), "{t}");
    }
}

#[test]
fn get_type_maps_shared_non_cycling_types() {
    assert_eq!(IcuActivity::get_type("Run").unwrap(), 3.into());
    assert_eq!(IcuActivity::get_type("VirtualRun").unwrap(), 3.into());
    assert_eq!(IcuActivity::get_type("Hike").unwrap(), 4.into());
    assert_eq!(IcuActivity::get_type("AlpineSki").unwrap(), 6.into());
    assert_eq!(IcuActivity::get_type("Walk").unwrap(), 8.into());
    assert_eq!(IcuActivity::get_type("Snowboard").unwrap(), 2.into());
    assert_eq!(IcuActivity::get_type("Workout").unwrap(), 0.into());
}

#[test]
fn get_type_rejects_unknown() {
    assert!(matches!(
        IcuActivity::get_type("Frobnicate"),
        Err(Error::BadRequest(_))
    ));
}

// ─── is_stub ────────────────────────────────────────────────────────────────

#[test]
fn is_stub_when_both_null() {
    let act: IcuActivity = serde_json::from_str(&stub_activity_json("10", "Ride")).unwrap();
    assert!(act.is_stub());
}

#[test]
fn is_stub_when_both_zero() {
    let act: IcuActivity = serde_json::from_str(
        r#"{"id":"10","type":"Ride","name":"x","start_date":"2026-01-02T10:00:00Z","distance":0.0,"moving_time":0,"elapsed_time":0,"total_elevation_gain":0.0,"icu_joules":0.0,"gear":null}"#,
    )
    .unwrap();
    assert!(act.is_stub());
}

#[test]
fn not_stub_when_distance_present() {
    assert!(!icu_activity(None).is_stub());
}

// ─── into_activity ──────────────────────────────────────────────────────────

#[tokio::test]
async fn into_activity_maps_fields() -> TbResult<()> {
    // No gear: `into_activity` is a field mapping (no store read for the gear),
    // so the canned store and facade are unused.
    let (mut _store, _facade) = setup();
    let act = icu_activity(None)
        .into_activity(UserId::from(1), &_facade, &mut _store)
        .await?;
    assert_eq!(
        act.id,
        ProviderActivityId::Icu(IcuActivityId::from("10")).into()
    );
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
    Ok(())
}

#[tokio::test]
async fn into_activity_resolves_existing_gear() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // The gear part exists: `part_by_source("icu:b1")` returns it.
    facade
        .expect_part_by_source()
        .returning(move |source, _store| {
            assert_eq!(source, "icu:b1");
            Ok(Some(part(
                PartId::from(7),
                UserId::from(1),
                1.into(),
                Some("icu:b1".into()),
            )))
        });
    let act = icu_activity(Some("b1"))
        .into_activity(UserId::from(1), &facade, &mut store)
        .await?;
    assert_eq!(act.gear, Some(PartId::from(7)));
    Ok(())
}

#[tokio::test]
async fn into_activity_drops_gear_when_not_imported() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // The gear part has not been imported: `part_by_source` is `None`, so the
    // activity's gear reference is dropped.
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    let act = icu_activity(Some("b1"))
        .into_activity(UserId::from(1), &facade, &mut store)
        .await?;
    assert!(act.gear.is_none());
    Ok(())
}

// ─── import_activity ────────────────────────────────────────────────────────

#[tokio::test]
async fn import_activity_stub_returns_none() -> TbResult<()> {
    let (mut store, facade) = setup();
    expect_token(&mut store);
    queue("/activity/10", &stub_activity_json("10", "Ride"));
    // A stub: skipped — no upsert fires.
    let summary = import_activity("10", UserId::from(1), &facade, &mut store).await?;
    assert!(summary.is_none());
    assert_eq!(requests(), vec!["/activity/10".to_string()]);
    Ok(())
}

#[tokio::test]
async fn import_activity_real_returns_summary() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    expect_token(&mut store);
    queue("/activity/10", &activity_json("10", "Ride", None));
    // A real activity: the facade's upsert returns a summary with the activity.
    facade
        .expect_upsert_activity()
        .returning(move |act, _user, _store| {
            let mut summary = Summary::default();
            summary.activities.upsert(act);
            Ok(summary)
        });
    let summary = import_activity("10", UserId::from(1), &facade, &mut store).await?;
    let summary = summary.expect("a real activity produces a summary");
    assert!(
        summary
            .activities
            .get(&ProviderActivityId::Icu(IcuActivityId::from("10")).into())
            .is_some()
    );
    Ok(())
}

#[tokio::test]
async fn import_activity_propagates_not_auth() {
    let (mut store, _facade) = setup();
    expect_token(&mut store);
    // The icu fetch 401s: the `NotAuth` is surfaced.
    queue_error("/activity/10", Error::NotAuth("revoked".to_string()));
    let res = import_activity("10", UserId::from(1), &_facade, &mut store).await;
    assert!(matches!(res, Err(Error::NotAuth(_))));
}
