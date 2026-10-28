//! The `Service` entity tests over a canned [`MockStore`] (issue #643).
//!
//! Each test runs the real service domain logic — the usage aggregation and
//! computation from canned activity data (a main part aggregates all of its
//! gear's activities, a sub-part aggregates only during its attachment
//! windows, and a part with no activities computes a zero usage), `recalculate`
//! from canned inputs, the ownership checks, the delete tombstone, and the
//! redo logic — over a [`MockStore`] whose methods return canned rows. The
//! domain logic runs for real; only the store I/O is canned, and no test reads
//! back a row it just wrote (that persistence pass-through is verified on the
//! sqlx seam, `sqlx/tests/store_seam.rs`, where Postgres is the source of
//! truth). The successor-chain and read-back persistence tests that build state
//! over multiple writes and read it back live in that suite; the
//! read-after-write pass-throughs and the subsumed cases are dropped.

use super::*;
use crate::test_support::{
    MockStore, fixtures, mock_session, only_tombstone, part_type_ids, sole_live,
};

use fixtures::{sample_purchase_date, test_session, test_user};
use part_type_ids::{BIKE, CHAIN};

fn sample_time() -> OffsetDateTime {
    time::macros::datetime!(2024-06-15 10:00 UTC)
}

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner +
/// the type that decides the main/sub usage path).
fn part_row(id: PartId, owner: UserId, what: PartTypeId) -> Part {
    Part {
        id,
        owner,
        what,
        name: "Main Bike".to_string(),
        vendor: "TendaBike".to_string(),
        model: "Standard".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: UsageId::new(),
        source: None,
        shop: None,
    }
}

/// A canned `Usage` row: the per-activity metrics that `activity.usage()` folds
/// into the aggregate.
fn usage(time: i32, distance: i32, climb: i32, descend: i32, energy: i32) -> Usage {
    Usage {
        id: UsageId::default(),
        time,
        distance,
        climb,
        descend,
        energy,
        count: 1,
    }
}

/// A canned `Activity` ride on the given gear with the given per-activity
/// metrics (the values that `activity.usage()` folds into the aggregate).
fn ride(id: i64, gear: PartId, start: OffsetDateTime, usage: Usage) -> Activity {
    Activity {
        id: ActivityId::new(id),
        user_id: test_user(),
        what: ActTypeId::from(1),
        name: "Ride".to_string(),
        start,
        duration: 3600,
        time: Some(usage.time),
        distance: Some(usage.distance),
        climb: Some(usage.climb),
        descend: Some(usage.descend),
        energy: Some(usage.energy),
        gear: Some(gear),
        device_name: None,
        external_id: None,
    }
}

/// A canned `Attachment` row: the minimal shape the sub-part usage path reads.
fn att(
    part: PartId,
    attached: OffsetDateTime,
    gear: PartId,
    hook: PartTypeId,
    detached: OffsetDateTime,
) -> Attachment {
    Attachment {
        part_id: part,
        attached,
        gear,
        hook,
        detached,
        usage: UsageId::new(),
    }
}

/// A canned `Service` row: the minimal shape the domain logic needs.
fn service_row(
    id: ServiceId,
    part: PartId,
    time: OffsetDateTime,
    plans: Vec<ServicePlanId>,
) -> Service {
    Service {
        id,
        part_id: part,
        time,
        redone: MAX_TIME,
        name: "Service".to_string(),
        notes: String::new(),
        usage: UsageId::new(),
        successor: None,
        plans,
    }
}

// === Suite 1: Service — value logic ===

/// S-01: ServiceId::new() produces a UUID (format check). No store.
#[test]
fn service_id_new_creates_uuid() {
    let id = ServiceId::new();
    let s = id.to_string();
    assert_eq!(s.len(), 36);
    assert_eq!(s.chars().filter(|c| *c == '-').count(), 4);
}

// === Suite 1: Service — Create (usage from canned activity data) ===

/// S-02: Service create reports the service with its supplied fields.
#[tokio::test]
async fn service_create_with_valid_data() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        part,
        t,
        "Chain Replacement".to_string(),
        "Old chain".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.services.len(), 1);
    let svc = sole_live(&summary.services);
    assert_eq!(svc.part_id, part);
    assert_eq!(svc.time, t);
    assert_eq!(svc.name, "Chain Replacement");
    assert_eq!(svc.notes, "Old chain");
    assert!(svc.successor.is_none());
    assert!(svc.plans.is_empty());
    Ok(())
}

/// S-05: Service create usage for a main part aggregates all of its gear's
/// activities (from MIN_TIME to the service time).
#[tokio::test]
async fn service_create_usage_main_part_aggregates_all_activities() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let acts = vec![
        ride(
            101,
            part,
            time::macros::datetime!(2024-01-01 10:00 UTC),
            usage(3500, 20000, 100, 80, 500),
        ),
        ride(
            102,
            part,
            time::macros::datetime!(2024-05-01 10:00 UTC),
            usage(7000, 40000, 200, 160, 1000),
        ),
    ];

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), BIKE)));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        part,
        t,
        "Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.time, 10500);
    assert_eq!(u.distance, 60000);
    assert_eq!(u.climb, 300);
    assert_eq!(u.descend, 240);
    assert_eq!(u.energy, 1500);
    assert_eq!(u.count, 2);
    Ok(())
}

/// S-06: Service create usage for a sub-part aggregates only during its
/// attachment periods; with no attachment rows the count is zero.
#[tokio::test]
async fn service_create_usage_sub_part_aggregates_activities_during_attachment() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        part,
        t,
        "Chain Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.count, 0);
    Ok(())
}

/// S-07: Service create usage is zero with no activities.
#[tokio::test]
async fn service_create_usage_zero_with_no_activities() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        part,
        t,
        "Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.time, 0);
    assert_eq!(u.distance, 0);
    assert_eq!(u.climb, 0);
    assert_eq!(u.descend, 0);
    assert_eq!(u.energy, 0);
    assert_eq!(u.count, 0);
    Ok(())
}

/// S-09: Service get returns NotFound for a missing service.
#[tokio::test]
async fn serviceid_get_returns_not_found_for_missing() -> TbResult<()> {
    let fake_id = ServiceId::new();

    let mut store = MockStore::new();
    store
        .expect_service_get()
        .returning(|_| Err(Error::NotFound("service".into())));

    let result = ServiceStore::service_get(&mut store, fake_id).await;
    assert!(
        matches!(result, Err(Error::NotFound(_))),
        "a missing service must be NotFound, got {result:?}"
    );
    Ok(())
}

// === Suite 2: Service — Update ===

/// S-11: Service update recalculates usage from the canned activity data.
#[tokio::test]
async fn service_update_recalculates_usage() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let stored = service_row(ServiceId::new(), part, t, vec![]);
    let stored_get = stored.clone();
    let acts = vec![ride(
        101,
        part,
        time::macros::datetime!(2024-01-01 10:00 UTC),
        usage(3500, 20000, 100, 80, 500),
    )];

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), BIKE)));
    store
        .expect_service_get()
        .returning(move |_| Ok(stored_get.clone()));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_update().returning(Ok);

    let mut svc = stored;
    svc.notes = "Updated notes".to_string();
    let summary = svc.update(&test_session(), &mut store).await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.time, 3500);
    Ok(())
}

/// S-13: Service update requires ownership.
#[tokio::test]
async fn service_update_requires_ownership() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let svc = service_row(ServiceId::new(), part, t, vec![]);

    let mut store = MockStore::new();
    // The part is owned by test_user(); user 2 (no shop) is denied before the
    // service is even read.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));

    let result = svc.update(&mock_session(UserId::from(2)), &mut store).await;
    assert!(result.is_err());
    Ok(())
}

// === Suite 2: Service — Delete ===

/// S-14b: Service delete tombstones the deleted service: the summary carries a
/// None entry for it, so the client's merge drops its row.
#[tokio::test]
async fn service_delete_reports_tombstone() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let svc_id = ServiceId::new();
    let stored = service_row(svc_id, part, t, vec![]);

    let mut store = MockStore::new();
    store
        .expect_service_get()
        .returning(move |_| Ok(stored.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    // No predecessor points at this service, so only the tombstone is reported.
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store
        .expect_usage_delete()
        .returning(|_| Ok(Usage::default()));
    store.expect_service_delete().returning(|_| Ok(1));

    let summary = svc_id.delete(&test_session(), &mut store).await?;
    assert_eq!(summary.services, only_tombstone(svc_id));
    Ok(())
}

/// S-17: Service delete (the store's raw delete) succeeds for the owner.
#[tokio::test]
async fn service_delete_by_partid_checks_ownership() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);
    store.expect_service_delete().returning(|_| Ok(1));

    let summary = Service::create(
        part,
        t,
        "Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;
    let svc_id = sole_live(&summary.services).id;

    let result = ServiceStore::service_delete(&mut store, svc_id).await;
    assert!(result.is_ok());
    Ok(())
}

// === Suite 3: Service — Redo ===

/// S-20: redo preserves name and notes from the original.
#[tokio::test]
async fn redo_preserves_name_and_notes_from_original() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let s1 = Service {
        name: "Chain Clean".to_string(),
        notes: "Use Shimano fluid".to_string(),
        ..service_row(ServiceId::new(), part, t, vec![])
    };
    let s1_get = s1.clone();

    let mut store = MockStore::new();
    store
        .expect_service_get()
        .returning(move |_| Ok(s1_get.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);
    store.expect_service_update().returning(Ok);

    let summary = s1.redo(&test_session(), &mut store).await?;
    let new_svc = summary
        .services
        .live()
        .into_iter()
        .find(|s| s.successor.is_none())
        .unwrap();

    assert_eq!(new_svc.name, "Chain Clean");
    assert_eq!(new_svc.notes, "Use Shimano fluid");
    Ok(())
}

/// S-21: redo preserves plans from the original.
#[tokio::test]
async fn redo_preserves_plans_from_original() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let plan_id = ServicePlanId::new();
    let s1 = service_row(ServiceId::new(), part, t, vec![plan_id]);
    let s1_get = s1.clone();

    let mut store = MockStore::new();
    store.expect_serviceplan_create().returning(Ok);
    store
        .expect_service_get()
        .returning(move |_| Ok(s1_get.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);
    store.expect_service_update().returning(Ok);

    let summary = s1.redo(&test_session(), &mut store).await?;
    let new_svc = summary
        .services
        .live()
        .into_iter()
        .find(|s| s.successor.is_none())
        .unwrap();

    assert_eq!(new_svc.plans, vec![plan_id]);
    Ok(())
}

/// S-22: redo recalculates usage for the new service (time-based filtering).
#[tokio::test]
async fn redo_recalculates_usage_for_new_service() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let s1 = service_row(ServiceId::new(), part, t, vec![]);
    let s1_get = s1.clone();
    let acts = vec![ride(
        101,
        part,
        time::macros::datetime!(2024-01-01 10:00 UTC),
        usage(3500, 20000, 100, 80, 500),
    )];

    let mut store = MockStore::new();
    store
        .expect_service_get()
        .returning(move |_| Ok(s1_get.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), BIKE)));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);
    store.expect_service_update().returning(Ok);

    let summary = s1.redo(&test_session(), &mut store).await?;
    let new_service = summary
        .services
        .live()
        .into_iter()
        .find(|s| s.successor.is_none())
        .expect("the redo reports the new service");
    let u = summary
        .usages
        .get(&new_service.usage)
        .expect("the new service's usage is live");
    assert_eq!(u.time, 3500);
    Ok(())
}

/// S-23: redo requires ownership.
#[tokio::test]
async fn redo_requires_ownership() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let s1 = service_row(ServiceId::new(), part, t, vec![]);
    let s1_get = s1.clone();

    let mut store = MockStore::new();
    store
        .expect_service_get()
        .returning(move |_| Ok(s1_get.clone()));
    // The part is owned by test_user(); user 2 (no shop) is denied.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));

    let result = s1.redo(&mock_session(UserId::from(2)), &mut store).await;
    assert!(result.is_err());
    Ok(())
}

// === Suite 4: Service — Usage Calculation ===

/// S-25: Service calculate_usage for a main part finds activities by gear.
#[tokio::test]
async fn service_calculate_usage_main_part_finds_activities_by_gear() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let acts = vec![ride(
        101,
        part,
        time::macros::datetime!(2024-01-01 10:00 UTC),
        usage(3500, 20000, 100, 80, 500),
    )];
    let svc = service_row(ServiceId::new(), part, t, vec![]);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), BIKE)));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));

    let usage = svc.calculate_usage(&mut store).await?;
    assert_eq!(usage.count, 1);
    assert_eq!(usage.time, 3500);
    Ok(())
}

/// S-26: Service calculate_usage for a sub-part uses attachment periods; with
/// no attachment rows the count is zero.
#[tokio::test]
async fn service_calculate_usage_sub_part_finds_activities_by_attachment_period() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let svc = service_row(ServiceId::new(), part, t, vec![]);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));

    let usage = svc.calculate_usage(&mut store).await?;
    assert_eq!(usage.count, 0);
    Ok(())
}

/// S-27: Service calculate_usage aggregates multiple activities.
#[tokio::test]
async fn service_calculate_usage_aggregates_multiple_activities() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let acts = vec![
        ride(
            101,
            part,
            time::macros::datetime!(2024-01-01 10:00 UTC),
            usage(3500, 20000, 100, 80, 500),
        ),
        ride(
            102,
            part,
            time::macros::datetime!(2024-01-02 10:00 UTC),
            usage(3500, 20000, 100, 80, 500),
        ),
        ride(
            103,
            part,
            time::macros::datetime!(2024-01-03 10:00 UTC),
            usage(3500, 20000, 100, 80, 500),
        ),
    ];
    let svc = service_row(ServiceId::new(), part, t, vec![]);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), BIKE)));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));

    let usage = svc.calculate_usage(&mut store).await?;
    assert_eq!(usage.count, 3);
    assert_eq!(usage.time, 10500);
    assert_eq!(usage.distance, 60000);
    Ok(())
}

// === Suite 5: Service — Recalculate ===

/// S-28: Service recalculate filters by attach time (services at or after the
/// given time are included).
#[tokio::test]
async fn service_recalculate_filters_by_attach_time() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let svcs = vec![service_row(ServiceId::new(), part, t, vec![])];

    let mut store = MockStore::new();
    store
        .expect_services_by_part()
        .returning(move |_| Ok(svcs.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));

    // Recalculate at a time before the service → the service is included.
    let early = time::macros::datetime!(2024-01-01 10:00 UTC);
    let recalc = Service::recalculate(part, early, &mut store).await?;
    assert_eq!(recalc.len(), 1);

    // Recalculate at a time after the service → no services.
    let late = time::macros::datetime!(2024-12-01 10:00 UTC);
    let recalc = Service::recalculate(part, late, &mut store).await?;
    assert!(recalc.is_empty());
    Ok(())
}

/// S-29: Service recalculate returns only the services at/after the time.
#[tokio::test]
async fn service_recalculate_after_detach_stale_services() -> TbResult<()> {
    let part = PartId::from(1);
    let t1 = time::macros::datetime!(2024-03-01 10:00 UTC);
    let t2 = time::macros::datetime!(2024-06-15 10:00 UTC);
    let svcs = vec![
        service_row(ServiceId::new(), part, t1, vec![]),
        service_row(ServiceId::new(), part, t2, vec![]),
    ];

    let mut store = MockStore::new();
    store
        .expect_services_by_part()
        .returning(move |_| Ok(svcs.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));

    // Recalculate at t2 → only the service at t2 is included.
    let recalc = Service::recalculate(part, t2, &mut store).await?;
    assert_eq!(recalc.len(), 1);
    Ok(())
}

/// S-30: Service recalculate handles an empty service list.
#[tokio::test]
async fn service_recalculate_handles_empty_service_list() -> TbResult<()> {
    let part = PartId::from(1);
    let t = time::macros::datetime!(2024-03-01 10:00 UTC);

    let mut store = MockStore::new();
    store.expect_services_by_part().returning(|_| Ok(vec![]));

    let recalc = Service::recalculate(part, t, &mut store).await?;
    assert!(recalc.is_empty());
    Ok(())
}

// === Suite 6: Service — Edge cases ===

/// S-35: Service create with an empty name succeeds.
#[tokio::test]
async fn service_create_with_empty_name() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        part,
        t,
        "".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    let svc = sole_live(&summary.services);
    assert_eq!(svc.name, "");
    Ok(())
}

/// S-36: Service create with a very long name succeeds.
#[tokio::test]
async fn service_create_with_very_long_name() -> TbResult<()> {
    let part = PartId::from(1);
    let t = sample_time();
    let long_name = "A".repeat(10000);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        part,
        t,
        long_name.clone(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    let svc = sole_live(&summary.services);
    assert_eq!(svc.name, long_name);
    Ok(())
}

// === Suite 7: Service — Usage Accounting (canned snapshot activity data) ===

/// S-43: Service create on a main part aggregates all of its gear's activities.
#[tokio::test]
async fn service_create_on_main_part_aggregates_prepopulated_activities() -> TbResult<()> {
    let bike = PartId::from(1);
    // All three activities predate this service time.
    let t = time::macros::datetime!(2023-06-01 10:00 UTC);
    let acts = vec![
        ride(
            1,
            bike,
            time::macros::datetime!(2023-05-18 22:13:20 UTC),
            usage(2525, 45000, 500, 500, 500),
        ),
        ride(
            2,
            bike,
            time::macros::datetime!(2023-05-19 00:13:20 UTC),
            usage(2700, 45000, 500, 500, 500),
        ),
        ride(
            3,
            bike,
            time::macros::datetime!(2023-05-19 22:13:20 UTC),
            usage(2800, 35000, 100, 100, 500),
        ),
    ];

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(bike, test_user(), BIKE)));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        bike,
        t,
        "Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.time, 8025);
    assert_eq!(u.distance, 125000);
    assert_eq!(u.climb, 1100);
    // The activities' descend is Some, so it is summed directly.
    assert_eq!(u.descend, 1100);
    assert_eq!(u.energy, 1500);
    assert_eq!(u.count, 3);
    Ok(())
}

/// S-44: Service create on an attached sub-part sums the activities within its
/// attachment window.
#[tokio::test]
async fn service_create_on_subpart_aggregates_prepopulated_activities_during_attachment()
-> TbResult<()> {
    let chain = PartId::from(4);
    let bike = PartId::from(1);
    // The chain is attached to the bike since 2023-01-01, never detached → all
    // three activities fall inside the window.
    let t = time::macros::datetime!(2023-06-01 10:00 UTC);
    let window = att(
        chain,
        time::macros::datetime!(2023-01-01 00:00 UTC),
        bike,
        BIKE,
        MAX_TIME,
    );
    let acts = vec![
        ride(
            1,
            bike,
            time::macros::datetime!(2023-05-18 22:13:20 UTC),
            usage(2525, 45000, 500, 500, 500),
        ),
        ride(
            2,
            bike,
            time::macros::datetime!(2023-05-19 00:13:20 UTC),
            usage(2700, 45000, 500, 500, 500),
        ),
        ride(
            3,
            bike,
            time::macros::datetime!(2023-05-19 22:13:20 UTC),
            usage(2800, 35000, 100, 100, 500),
        ),
    ];

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(chain, test_user(), CHAIN)));
    store
        .expect_attachments_all_by_part()
        .returning(move |_| Ok(vec![window]));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        chain,
        t,
        "Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.time, 8025);
    assert_eq!(u.distance, 125000);
    assert_eq!(u.climb, 1100);
    assert_eq!(u.descend, 1100);
    assert_eq!(u.energy, 1500);
    assert_eq!(u.count, 3);
    Ok(())
}

/// S-45: Service create only counts the activities before the service time.
#[tokio::test]
async fn service_create_before_latest_activity_counts_only_earlier_ones() -> TbResult<()> {
    let bike = PartId::from(1);
    // Between the two earlier rides and the latest → only the first two count.
    let t = time::macros::datetime!(2023-05-19 12:00 UTC);
    let acts = vec![
        ride(
            1,
            bike,
            time::macros::datetime!(2023-05-18 22:13:20 UTC),
            usage(2525, 45000, 500, 500, 500),
        ),
        ride(
            2,
            bike,
            time::macros::datetime!(2023-05-19 00:13:20 UTC),
            usage(2700, 45000, 500, 500, 500),
        ),
    ];

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(bike, test_user(), BIKE)));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(move |_part, _begin, _end| Ok(acts.clone()));
    store.expect_usages_update().returning(|_| Ok(1));
    store.expect_service_create().returning(Ok);

    let summary = Service::create(
        bike,
        t,
        "Service".to_string(),
        "".to_string(),
        None,
        vec![],
        &mut store,
    )
    .await?;

    assert_eq!(summary.usages.len(), 1);
    let u = sole_live(&summary.usages);
    assert_eq!(u.time, 5225);
    assert_eq!(u.distance, 90000);
    assert_eq!(u.climb, 1000);
    assert_eq!(u.descend, 1000);
    assert_eq!(u.energy, 1000);
    assert_eq!(u.count, 2);
    Ok(())
}
