//! The `Activity` entity tests over a canned [`MockStore`] (issue #644).
//!
//! Each test runs the real activity domain logic over a [`MockStore`] whose
//! methods return canned rows. Two kinds of logic live here:
//!
//! - **Value logic** (`ActivityId::new`, `Activity::usage` extraction, including
//!   the descend/time defaults) runs store-free, as plain unit tests.
//! - **Accounting and orchestration** run for real over canned store inputs,
//!   with **no read-back of a row just written**: upsert/delete accounting is
//!   computed in memory from canned attachments and usages, `csv2descend`
//!   parses numbers against canned `get_by_user_and_time` lookups, and
//!   `set_default_part` delegates the gear assignment to a canned
//!   `activity_set_gear_if_null` and registers the returned rides. The
//!   persistence pass-throughs it drops (the `find` / `get_all` /
//!   `get_by_user_and_time` reads, the `activity_update` field-preservation
//!   rule) and the `rescan_all` full-rebuild read-backs are verified on the
//!   sqlx seam, `sqlx/tests/store_seam.rs`, where Postgres is the source of
//!   truth.

use super::*;
use crate::test_support::{MockStore, fixtures, mock_session, only_tombstone, part_type_ids};

use fixtures::{test_session, test_user};
use part_type_ids::BIKE;

fn activity_start() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

fn later_start() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700100000).unwrap()
}

fn sample_activity() -> Activity {
    Activity {
        id: ActivityId::new(1),
        user_id: test_user(),
        what: ActTypeId::from(1),
        name: "Morning Ride".to_string(),
        start: activity_start(),
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: Some(300),
        energy: Some(1000),
        gear: None,
        device_name: Some("Garmin Edge".to_string()),
        external_id: Some("garmin_12345".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Canned row builders and accounting wiring
// ---------------------------------------------------------------------------

/// A fixed usage id (the domain mints v7 UUIDs at runtime; tests need a stable
/// value to match the canned `usage_get` against).
fn usage_id(n: u128) -> UsageId {
    UsageId::from(uuid::Uuid::from_u128(n))
}

/// A canned `Usage` row.
fn usage_row(
    id: UsageId,
    time: i32,
    distance: i32,
    climb: i32,
    descend: i32,
    energy: i32,
    count: i32,
) -> Usage {
    Usage {
        id,
        time,
        distance,
        climb,
        descend,
        energy,
        count,
    }
}

/// The prepopulated bike-set baseline the fixture carries (the value the
/// activity accounting adds to and subtracts from).
fn baseline_usage() -> Usage {
    usage_row(UsageId::default(), 8025, 125000, 1100, 1100, 1500, 3)
}

/// A canned `Part` row: the minimal shape the accounting needs (id + owner +
/// type + its usage id). `purchase` and `last_used` sit well before the
/// activity start, so `update_timestamps` bumps only `last_used`.
fn part_row(id: PartId, owner: UserId, what: PartTypeId, usage: UsageId) -> Part {
    Part {
        id,
        owner,
        what,
        name: "Main Bike".to_string(),
        vendor: "TendaBike".to_string(),
        model: "Standard".to_string(),
        purchase: time::macros::datetime!(2023-01-01 00:00 UTC),
        last_used: time::macros::datetime!(2023-01-01 00:00 UTC),
        disposed_at: None,
        usage,
        source: None,
        shop: None,
    }
}

/// A canned `Attachment` row: a sub-part attached to `gear` since 2023-01-01.
fn att(part: PartId, gear: PartId, usage: UsageId) -> Attachment {
    Attachment {
        part_id: part,
        attached: time::macros::datetime!(2023-01-01 00:00 UTC),
        gear,
        hook: PartTypeId::from(1),
        detached: MAX_TIME,
        usage,
    }
}

/// Wire the `MockStore` to serve the `register` accounting for `gear`: the
/// canned attachments at the gear, the part reads (each with its own usage
/// id), no services, the `baseline` usage for every id, and the bulk usage
/// write. The domain then computes the new usage in memory from these canned
/// inputs — no test reads back a row it just wrote.
fn wire_register_accounting(store: &mut MockStore, attachments: Vec<Attachment>, baseline: Usage) {
    store
        .expect_attachment_get_by_gear_and_time()
        .returning(move |_gear, _start| Ok(attachments.clone()));
    store.expect_partid_get_part().returning(move |id| {
        Ok(part_row(
            id,
            test_user(),
            PartTypeId::from(1),
            usage_id(i32::from(id) as u128),
        ))
    });
    store.expect_part_update().returning(Ok);
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_usage_get().returning(move |id| {
        Ok(Some(Usage {
            id,
            ..baseline.clone()
        }))
    });
    store.expect_usages_update().returning(|_| Ok(1));
}

// === Suite 1: ActivityId and usage — value logic (no store) ===

/// ActivityId::new creates an ActivityId with the given value
#[test]
fn activityid_new_creates_with_value() {
    let id = ActivityId::new(42);
    assert_eq!(format!("{}", id), "42");
}

/// Activity::usage extracts Usage from activity
#[test]
fn activity_usage_returns_usage() {
    let act = sample_activity();
    let usage = act.usage();

    assert_eq!(usage.time, 3500);
    assert_eq!(usage.distance, 50000);
    assert_eq!(usage.climb, 500);
    assert_eq!(usage.descend, 300);
    assert_eq!(usage.energy, 1000);
    assert_eq!(usage.count, 1);
}

/// Activity::usage defaults descend to climb when descend is None
#[test]
fn activity_usage_defaults_descend_to_climb() {
    let mut act = sample_activity();
    act.descend = None;
    let usage = act.usage();

    assert_eq!(usage.descend, 500); // climb value
}

/// Activity::usage defaults time to 0 when time is None
#[test]
fn activity_usage_defaults_time_to_zero() {
    let mut act = sample_activity();
    act.time = None;
    let usage = act.usage();

    assert_eq!(usage.time, 0);
}

/// Activity::usage returns all metrics when all fields are Some
#[test]
fn activity_usage_preserves_all_some_values() {
    let act = sample_activity();
    let usage = act.usage();

    assert_eq!(usage.time, 3500);
    assert_eq!(usage.distance, 50000);
    assert_eq!(usage.climb, 500);
    assert_eq!(usage.descend, 300);
    assert_eq!(usage.energy, 1000);
    assert_eq!(usage.count, 1);
}

/// Activity::usage defaults all Option fields to zero when None
#[test]
fn activity_usage_all_defaults_to_zero() {
    let mut act = sample_activity();
    act.time = None;
    act.distance = None;
    act.climb = None;
    act.descend = None;
    act.energy = None;
    let usage = act.usage();

    assert_eq!(usage.time, 0);
    assert_eq!(usage.distance, 0);
    assert_eq!(usage.climb, 0);
    assert_eq!(usage.descend, 0); // descend defaults to climb which is also None → 0
    assert_eq!(usage.energy, 0);
    assert_eq!(usage.count, 1);
}

// === Suite 2: ActivityId read — authorization ===

/// ActivityId::read rejects cross-user access: the stored ride is owned by
/// user 1, so user 2's read is `Forbidden` before any accounting.
#[tokio::test]
async fn activityid_read_rejects_cross_user() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));

    let other_session = mock_session(UserId::from(2));
    let result = ActivityId::new(1).read(&other_session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === Suite 3: upsert — create and replace ===

/// Activity::upsert creates a new activity (a null-gear ride reports only
/// itself).
#[tokio::test]
async fn activity_upsert_creates_new() -> TbResult<()> {
    let mut store = MockStore::new();
    let act = Activity {
        name: "New Activity".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.expect_activity_read_by_id().returning(|_| Ok(None));
    store.expect_activity_create().returning(Ok);

    let summary = act.upsert(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(
        summary.activities.get(&ActivityId::new(1)).unwrap().name,
        "New Activity"
    );
    Ok(())
}

/// Creating a new activity for the prepopulated bike (id 1) accounts for the
/// bike and all attached parts: the summary contains the bike, every part
/// attached to it, and the updated usage of each of them, computed in memory
/// from the canned attachments and usages.
#[tokio::test]
async fn activity_upsert_creates_new_accounts_bike_and_attached_parts() -> TbResult<()> {
    let bike = PartId::from(1);
    let act = Activity {
        name: "New Ride".to_string(),
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        gear: Some(bike),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let expected_act = act.clone();

    let mut store = MockStore::new();
    store.expect_activity_read_by_id().returning(|_| Ok(None));
    store.expect_activity_create().returning(Ok);
    // The bike's five attached sub-parts (front/rear wheel, chain, tires).
    let attachments = (2..=6)
        .map(|p| att(PartId::from(p), bike, usage_id(100 + p as u128)))
        .collect();
    wire_register_accounting(&mut store, attachments, baseline_usage());

    let summary = act.upsert(&test_session(), &mut store).await?;

    // the new activity is reported
    let mut expected = IdKeyedMap::default();
    expected.upsert(expected_act);
    assert_eq!(summary.activities, expected);

    // the bike and all attached parts: front wheel, rear wheel, chain, tires
    let part_ids: HashSet<PartId> = summary.parts.live().into_iter().map(|p| p.id).collect();
    assert_eq!(
        part_ids,
        [1, 2, 3, 4, 5, 6].into_iter().map(PartId::from).collect()
    );
    // last_used of the bike is bumped to the activity start
    let bike_part = summary
        .parts
        .live()
        .into_iter()
        .find(|p| p.id == bike)
        .unwrap();
    assert_eq!(bike_part.last_used, round_time(activity_start()));

    // every affected usage (6 part usages + 5 attachment usages) is increased
    // by the activity metrics (8025+1000, 125000+10000, 1100+100, 1100+100,
    // 1500+200, 3+1); descend is None in the activity, so it falls back to climb
    assert_eq!(summary.usages.len(), 11);
    let mut expected_usage = Usage {
        id: UsageId::default(),
        time: 9025,
        distance: 135000,
        climb: 1200,
        descend: 1200,
        energy: 1700,
        count: 4,
    };
    for u in summary.usages.live() {
        expected_usage.id = u.id;
        assert_eq!(u, expected_usage, "unexpected usage for {}", u.id);
    }
    Ok(())
}

/// Activity::upsert updates an existing activity (the replace path: the old
/// ride is unregistered and the new one registered).
#[tokio::test]
async fn activity_upsert_updates_existing() -> TbResult<()> {
    let mut store = MockStore::new();
    let act = sample_activity();
    let stored = act.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(stored.clone())));
    store.expect_activity_update().returning(Ok);

    let modified = Activity {
        name: "Updated Activity".to_string(),
        ..act
    };
    let summary = modified.upsert(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(
        summary.activities.get(&ActivityId::new(1)).unwrap().name,
        "Updated Activity"
    );
    Ok(())
}

/// Activity::upsert preserves the caller's activity id (a custom,
/// non-sequential id is kept).
#[tokio::test]
async fn activity_upsert_preserves_original_id() -> TbResult<()> {
    let mut store = MockStore::new();
    let act_id = ActivityId::new(42);
    let act = Activity {
        id: act_id,
        name: "Custom ID Activity".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.expect_activity_read_by_id().returning(|_| Ok(None));
    store.expect_activity_create().returning(Ok);

    let summary = act.upsert(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.get(&act_id).unwrap().id, act_id);
    Ok(())
}

// === Suite 4: delete ===

/// Deleting an activity for the prepopulated bike (id 1) reverts the
/// accounting of the create: the summary contains the bike, every part
/// attached to it, and the usage of each of them back at its previous value,
/// computed in memory from the canned attachments and usages.
#[tokio::test]
async fn activity_delete_reverts_bike_and_attached_part_usage() -> TbResult<()> {
    let bike = PartId::from(1);
    let act = Activity {
        name: "New Ride".to_string(),
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        gear: Some(bike),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    let mut store = MockStore::new();
    // The ride is stored and accounted, so the baseline already includes it.
    let stored = act.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(stored.clone())));
    store.expect_activity_delete().returning(|_| Ok(1));
    let attachments = (2..=6)
        .map(|p| att(PartId::from(p), bike, usage_id(100 + p as u128)))
        .collect();
    // The baseline is the post-create value (9025); the subtract reverts it.
    wire_register_accounting(
        &mut store,
        attachments,
        usage_row(UsageId::default(), 9025, 135000, 1200, 1200, 1700, 4),
    );

    let summary = ActivityId::new(1)
        .delete(&test_session(), &mut store)
        .await?;

    // the deleted activity is reported as a None tombstone
    assert_eq!(summary.activities, only_tombstone(act.id));

    // the bike and all attached parts are affected again
    let part_ids: HashSet<PartId> = summary.parts.live().into_iter().map(|p| p.id).collect();
    assert_eq!(
        part_ids,
        [1, 2, 3, 4, 5, 6].into_iter().map(PartId::from).collect()
    );

    // every affected usage is back at its prepopulated baseline
    assert_eq!(summary.usages.len(), 11);
    let mut expected_usage = Usage {
        id: UsageId::default(),
        time: 8025,
        distance: 125000,
        climb: 1100,
        descend: 1100,
        energy: 1500,
        count: 3,
    };
    for u in summary.usages.live() {
        expected_usage.id = u.id;
        assert_eq!(u, expected_usage, "unexpected usage for {}", u.id);
    }
    Ok(())
}

/// Activity::delete unregisters a null-gear activity and reports it as a
/// tombstone.
#[tokio::test]
async fn activity_delete_returns_summary() -> TbResult<()> {
    let mut store = MockStore::new();
    let act = sample_activity();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(act.clone())));
    store.expect_activity_delete().returning(|_| Ok(1));

    let summary = ActivityId::new(1)
        .delete(&test_session(), &mut store)
        .await?;
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.is_tombstoned(&ActivityId::new(1)));
    Ok(())
}

/// Activity::delete returns forbidden for a non-owner session: the stored
/// ride is owned by user 1, so user 99's delete is `Forbidden`.
#[tokio::test]
async fn activity_delete_rejects_non_owner() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));

    let other_session = mock_session(UserId::from(99));
    let result = ActivityId::new(1).delete(&other_session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === Suite 5: update ===

/// Activity::update returns a summary containing the updated activity.
#[tokio::test]
async fn activity_update_returns_summary() -> TbResult<()> {
    let mut store = MockStore::new();
    let act = sample_activity();
    let stored = act.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(stored.clone())));
    store.expect_activity_update().returning(Ok);

    let modified = Activity {
        name: "Modified Ride".to_string(),
        ..act
    };
    let summary = modified.update(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    Ok(())
}

/// Activity::update on a missing activity returns NotFound.
#[tokio::test]
async fn activity_update_missing_activity_returns_not_found() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_activity_read_by_id().returning(|_| Ok(None));

    let fake = Activity {
        name: "Ghost Activity".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let result = fake.update(&test_session(), &mut store).await;
    assert!(result.is_err());
    Ok(())
}

// === Suite 6: categories (mapping) ===

/// Activity::categories maps the user's activity types to unique gear types:
/// two rides of the same type collapse to one gear type (the bike).
#[tokio::test]
async fn activity_categories_returns_unique_gear_types() -> TbResult<()> {
    let mut store = MockStore::new();
    let acts = vec![
        sample_activity(),
        Activity {
            id: ActivityId::new(2),
            name: "Another Ride".to_string(),
            start: later_start(),
            ..sample_activity()
        },
    ];
    store.expect_get_all().returning(move |_| Ok(acts.clone()));

    let categories = Activity::categories(&test_session(), &mut store).await?;
    assert_eq!(categories, HashSet::from([BIKE]));
    Ok(())
}

/// Activity::categories is empty for a user with no activities.
#[tokio::test]
async fn activity_categories_empty_for_no_activities() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_get_all().returning(|_| Ok(vec![]));

    let categories = Activity::categories(&mock_session(UserId::from(99)), &mut store).await?;
    assert!(categories.is_empty());
    Ok(())
}

// === Suite 7: register (accounting) ===

/// Activity registration skips when gear is None: no parts are touched.
#[tokio::test]
async fn activity_register_no_gear_does_not_update_parts() -> TbResult<()> {
    let mut store = MockStore::new();
    let act = sample_activity(); // gear = None

    let summary = act.register(Factor::Add, &mut store).await?;

    assert_eq!(summary.parts.len(), 0);
    assert_eq!(summary.activities.len(), 1);
    Ok(())
}

/// Activity registration touches only the gear: a detached sub-part (not
/// attached at the start) is skipped.
#[tokio::test]
async fn activity_register_skips_detached_parts() -> TbResult<()> {
    let bike = PartId::from(1);
    let mut store = MockStore::new();
    // No sub-part is attached to the bike at the start; only the gear itself.
    wire_register_accounting(&mut store, vec![], Usage::default());

    let act = Activity {
        gear: Some(bike),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let summary = act.register(Factor::Add, &mut store).await?;

    // Only the gear (bike) should be in parts, not a detached sub-part.
    assert!(summary.parts.live().into_iter().all(|p| p.id == bike));
    Ok(())
}

/// Activity with zero duration is still registered (a null-gear ride reports
/// only itself).
#[tokio::test]
async fn activity_with_zero_duration_still_registered() -> TbResult<()> {
    let mut store = MockStore::new();
    let mut act = sample_activity();
    act.duration = 0;

    let summary = act.register(Factor::Add, &mut store).await?;

    assert_eq!(summary.activities.len(), 1);
    assert_eq!(summary.parts.len(), 0); // No gear, so no parts updated
    Ok(())
}

/// Activity with only climb (no other metrics) produces a valid usage: the
/// pure extraction is asserted, then the null-gear ride is registered.
#[tokio::test]
async fn activity_with_only_climb_no_other_metrics() -> TbResult<()> {
    let act = Activity {
        name: "Climb Only".to_string(),
        time: None,
        distance: None,
        climb: Some(1000),
        descend: None, // should default to climb
        energy: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    let usage = act.usage();
    assert_eq!(usage.time, 0); // time was None
    assert_eq!(usage.distance, 0); // distance was None
    assert_eq!(usage.climb, 1000); // climb is Some(1000)
    assert_eq!(usage.descend, 1000); // descend defaults to climb (1000)
    assert_eq!(usage.energy, 0); // energy was None
    assert_eq!(usage.count, 1);

    let mut store = MockStore::new();
    let summary = act.register(Factor::Add, &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    Ok(())
}

// === Suite 8: CSV descend import (number parsing) ===

/// csv2descend parses the German date/title/descend column names and updates
/// each matching ride.
#[tokio::test]
async fn csv2descend_parses_german_date_format() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_get_by_user_and_time().returning(|_uid, t| {
        Ok(Activity {
            start: t,
            ..sample_activity()
        })
    });
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));
    store.expect_activity_update().returning(Ok);

    let csv_data = "Datum,Titel,Negativer Höhenunterschied\n\
                    2023-11-14 22:13:20,Ride1,300\n\
                    2023-11-15 00:13:20,Ride2,200";
    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(result.1.good.len(), 2); // 2 good records
    Ok(())
}

/// csv2descend parses the English title field alias.
#[tokio::test]
async fn csv2descend_parses_english_title_field() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_get_by_user_and_time().returning(|_uid, t| {
        Ok(Activity {
            start: t,
            ..sample_activity()
        })
    });
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));
    store.expect_activity_update().returning(Ok);

    let csv_data = "Date,Title,Total Descent\n2023-11-14 22:13:20,English Ride,300";
    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(result.1.good.len(), 1); // 1 good record
    Ok(())
}

/// csv2descend strips the German thousands separator before parsing the
/// descend number ("1.234" → 1234).
#[tokio::test]
async fn csv2descend_skips_german_decimal_format() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_get_by_user_and_time().returning(|_uid, t| {
        Ok(Activity {
            start: t,
            ..sample_activity()
        })
    });
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));
    store.expect_activity_update().returning(Ok);

    let csv_data = "Datum,Titel,Negativer Höhenunterschied\n2023-11-14 22:13:20,Ride,1.234";
    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    // "1.234" → stripped to "1234" → parsed as 1234
    assert_eq!(result.1.good.len(), 1); // record parses successfully
    Ok(())
}

/// csv2descend fails the import when a descend value is not a number.
#[tokio::test]
async fn csv2descend_returns_good_and_bad_lists() -> TbResult<()> {
    let mut store = MockStore::new();
    // The first row matches; the second row's descend is not a number.
    store.expect_get_by_user_and_time().returning(|_uid, t| {
        Ok(Activity {
            start: t,
            ..sample_activity()
        })
    });
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));
    store.expect_activity_update().returning(Ok);

    let csv_data = "Datum,Titel,Negativer Höhenunterschied\n\
                    2023-11-14 22:13:20,Valid Ride,300\n\
                    2023-11-14 23:13:20,Bad Ride,not_a_number";
    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await;

    // The invalid descend value fails the whole import.
    assert!(result.is_err());
    Ok(())
}

/// csv2descend reports the rows it matched and skipped as a `DescendReport`
/// (the handler's response body): human-readable descriptions, one per row.
#[tokio::test]
async fn csv2descend_reports_matched_and_skipped_rows() -> TbResult<()> {
    let mut store = MockStore::new();
    // The first row matches a ride at its time; the second has none.
    store.expect_get_by_user_and_time().returning(|_uid, t| {
        if t.unix_timestamp() == activity_start().unix_timestamp() {
            Ok(Activity {
                id: ActivityId::new(301),
                name: "Matched Ride".to_string(),
                start: activity_start(),
                ..sample_activity()
            })
        } else {
            Err(Error::NotFound("no activity".into()))
        }
    });
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));
    store.expect_activity_update().returning(Ok);

    let csv_data = "Datum,Titel,Negativer Höhenunterschied\n\
                    2023-11-14 22:13:20,Matched Ride,300\n\
                    2023-12-01 08:00:00,Phantom Ride,400";
    let (summary, report) =
        Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(
        report,
        DescendReport {
            good: vec!["Matched Ride at 2023-11-14 22:13:20".to_string()],
            bad: vec!["Phantom Ride at 2023-12-01 08:00:00".to_string()],
        }
    );
    // The matched activity carries the new descend value; the phantom row
    // touched nothing.
    assert_eq!(
        summary
            .activities
            .get(&ActivityId::new(301))
            .unwrap()
            .descend,
        Some(300)
    );
    Ok(())
}

/// csv2descend updates every valid record (one `match_and_update` per row).
#[tokio::test]
async fn csv2descend_calls_match_and_update_for_each_record() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_get_by_user_and_time().returning(|_uid, t| {
        Ok(Activity {
            start: t,
            ..sample_activity()
        })
    });
    store
        .expect_activity_read_by_id()
        .returning(|_| Ok(Some(sample_activity())));
    store.expect_activity_update().returning(Ok);

    let csv_data = "Datum,Titel,Negativer Höhenunterschied\n\
                    2023-11-14 22:13:20,Ride One,300\n\
                    2023-11-15 00:13:20,Ride Two,200";
    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(result.1.good.len(), 2); // Both records parsed and updated
    Ok(())
}

// === Suite 9: set_default_part (orchestration) ===

/// set_default_part assigns the gear to the matching null-gear activity and
/// registers it: the gear assignment delegates to the canned
/// `activity_set_gear_if_null`, and the returned ride is accounted.
#[tokio::test]
async fn set_default_part_assigns_gear_to_matching_activities() -> TbResult<()> {
    let bike = PartId::from(1);
    let mut store = MockStore::new();
    // The store assigns the gear to the matching null-gear activity.
    store
        .expect_activity_set_gear_if_null()
        .returning(move |_uid, _types, _part| {
            Ok(vec![Activity {
                gear: Some(bike),
                device_name: None,
                external_id: None,
                ..sample_activity()
            }])
        });
    wire_register_accounting(&mut store, vec![], Usage::default());

    let summary = Activity::set_default_part(bike, &test_session(), &mut store).await?;

    assert_eq!(
        summary.activities.get(&ActivityId::new(1)).unwrap().gear,
        Some(bike)
    );
    assert_eq!(
        summary
            .parts
            .live()
            .into_iter()
            .map(|p| p.id)
            .collect::<HashSet<_>>(),
        HashSet::from([bike])
    );
    Ok(())
}

/// set_default_part only assigns the gear to matching types: a running
/// activity (type 3) is outside the bike's act_types, so none is assigned.
#[tokio::test]
async fn set_default_part_only_affected_matching_types() -> TbResult<()> {
    let bike = PartId::from(1);
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(bike, test_user(), BIKE, usage_id(1))));
    // No activity matches the bike's act_types.
    store
        .expect_activity_set_gear_if_null()
        .returning(|_uid, _types, _part| Ok(vec![]));

    let summary = Activity::set_default_part(bike, &test_session(), &mut store).await?;

    assert!(summary.activities.is_empty());
    assert!(summary.usages.is_empty());
    Ok(())
}

/// set_default_part does not override an existing gear assignment: an
/// activity that already has a gear is not reassigned.
#[tokio::test]
async fn set_default_part_does_not_override_existing_gear() -> TbResult<()> {
    let bike = PartId::from(1);
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(bike, test_user(), BIKE, usage_id(1))));
    // The only activity already has a gear, so it is not reassigned.
    store
        .expect_activity_set_gear_if_null()
        .returning(|_uid, _types, _part| Ok(vec![]));

    let summary = Activity::set_default_part(bike, &test_session(), &mut store).await?;

    assert!(summary.activities.is_empty());
    Ok(())
}

/// set_default_part reports zero usage when no activities match.
#[tokio::test]
async fn set_default_part_empty_returns_zero_usage() -> TbResult<()> {
    let bike = PartId::from(1);
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(bike, test_user(), BIKE, usage_id(1))));
    store
        .expect_activity_set_gear_if_null()
        .returning(|_uid, _types, _part| Ok(vec![]));

    let summary = Activity::set_default_part(bike, &test_session(), &mut store).await?;

    assert_eq!(summary.usages.len(), 0);
    Ok(())
}

/// set_default_part assigns the gear and registers the newly assigned
/// activity: the bike usage is incremented by the ride's metrics, computed in
/// memory from the canned attachments and usages.
#[tokio::test]
async fn set_default_part_accounts_usage_for_newly_assigned_activities() -> TbResult<()> {
    let bike = PartId::from(1);
    let act = Activity {
        id: ActivityId::new(101),
        name: "Unassigned Ride".to_string(),
        start: time::macros::datetime!(2023-05-20 10:00 UTC),
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        gear: Some(bike),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    let mut store = MockStore::new();
    // The store assigns the gear to the unassigned ride.
    store
        .expect_activity_set_gear_if_null()
        .returning(move |_uid, _types, _part| Ok(vec![act.clone()]));
    // The bike and its attached sub-parts, at the prepopulated baseline.
    let attachments = (2..=6)
        .map(|p| att(PartId::from(p), bike, usage_id(100 + p as u128)))
        .collect();
    wire_register_accounting(&mut store, attachments, baseline_usage());

    let summary = Activity::set_default_part(bike, &test_session(), &mut store).await?;

    assert_eq!(
        summary.activities.get(&ActivityId::new(101)).unwrap().gear,
        Some(bike)
    );
    // The bike usage row is incremented by the newly assigned activity:
    // 8025+1000, 125000+10000, 1100+100, 1100+100 (descend None → climb),
    // 1500+200, 3+1.
    assert_eq!(summary.usages.len(), 11);
    let mut expected_usage = Usage {
        id: UsageId::default(),
        time: 9025,
        distance: 135000,
        climb: 1200,
        descend: 1200,
        energy: 1700,
        count: 4,
    };
    for u in summary.usages.live() {
        expected_usage.id = u.id;
        assert_eq!(u, expected_usage, "unexpected usage for {}", u.id);
    }
    Ok(())
}

/// set_default_part requires ownership: a ride's gear is only set when the
/// session owns the gear part.
#[tokio::test]
async fn set_default_part_requires_ownership() -> TbResult<()> {
    let bike = PartId::from(1);
    let mut store = MockStore::new();
    // The part is owned by user 2; user 1 (the session) is denied.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(bike, UserId::from(2), BIKE, usage_id(1))));

    let result = Activity::set_default_part(bike, &test_session(), &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === Suite 10: replace (gear change) ===

/// replace() on the same activity with a changed gear touches both the old
/// and the new gear: the old is unregistered and the new is registered.
#[tokio::test]
async fn replace_changes_affected_parts() -> TbResult<()> {
    let bike1 = PartId::from(1);
    let bike2 = PartId::from(2);
    let old_act = Activity {
        name: "Road Ride".to_string(),
        gear: Some(bike1),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let new_act = Activity {
        name: "MTB Ride".to_string(),
        gear: Some(bike2),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    let mut store = MockStore::new();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(old_act.clone())));
    store.expect_activity_update().returning(Ok);
    // The old gear (bike1) is unregistered, the new gear (bike2) registered;
    // neither has attached sub-parts.
    wire_register_accounting(&mut store, vec![], Usage::default());

    let summary = new_act.update(&test_session(), &mut store).await?;

    // Both the old and the new gear are in the summary.
    let part_ids: HashSet<PartId> = summary.parts.live().into_iter().map(|p| p.id).collect();
    assert_eq!(part_ids, HashSet::from([bike1, bike2]));
    Ok(())
}

/// replace() moves usage between the old and the new gear: the old bike's
/// usage reverts and the new bike's usage holds exactly the moved ride.
#[tokio::test]
async fn replace_gear_change_moves_usage_between_bikes() -> TbResult<()> {
    let bike1 = PartId::from(1);
    let bike2 = PartId::from(2);
    // The ride is registered on bike1 (its usage already accounts the ride).
    let old_act = Activity {
        name: "Ride".to_string(),
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        gear: Some(bike1),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let new_act = Activity {
        name: "Ride (replaced)".to_string(),
        gear: Some(bike2),
        ..old_act.clone()
    };

    let mut store = MockStore::new();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(old_act.clone())));
    store.expect_activity_update().returning(Ok);
    store
        .expect_attachment_get_by_gear_and_time()
        .returning(|_gear, _start| Ok(vec![]));
    store.expect_partid_get_part().returning(move |id| {
        Ok(part_row(
            id,
            test_user(),
            BIKE,
            usage_id(i32::from(id) as u128),
        ))
    });
    store.expect_part_update().returning(Ok);
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    // bike1's usage already includes the ride (9025); bike2 starts at zero.
    store.expect_usage_get().returning(move |id| {
        if id == usage_id(1) {
            Ok(Some(usage_row(id, 9025, 135000, 1200, 1200, 1700, 4)))
        } else {
            Ok(Some(usage_row(id, 0, 0, 0, 0, 0, 0)))
        }
    });
    store.expect_usages_update().returning(|_| Ok(2));

    let summary = new_act.update(&test_session(), &mut store).await?;

    // The old bike's usage is back at the prepopulated baseline …
    let reverted = summary.usages.get(&usage_id(1)).unwrap();
    assert_eq!(reverted.time, 8025);
    assert_eq!(reverted.distance, 125000);
    assert_eq!(reverted.climb, 1100);
    assert_eq!(reverted.descend, 1100);
    assert_eq!(reverted.energy, 1500);
    assert_eq!(reverted.count, 3);
    // … and the new bike's usage holds exactly the moved ride.
    let moved = summary.usages.get(&usage_id(2)).unwrap();
    assert_eq!(moved.time, 1000);
    assert_eq!(moved.distance, 10000);
    assert_eq!(moved.climb, 100);
    assert_eq!(moved.descend, 100);
    assert_eq!(moved.energy, 200);
    assert_eq!(moved.count, 1);
    Ok(())
}
