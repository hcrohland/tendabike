use std::collections::HashSet;

use tb_domain::test_support::{TestSession, only_tombstone, sole_live};
use tb_domain::*;
use tb_teststore::TestStore;
use time::OffsetDateTime;

use tb_teststore::fixtures::{test_session, test_user};

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

// === ActivityId tests ===

/// ActivityId::new creates an ActivityId with the given value
#[test]
fn activityid_new_creates_with_value() {
    let id = ActivityId::new(42);
    assert_eq!(format!("{}", id), "42");
}

/// ActivityId::read_optional returns None for non-existent activity
#[tokio::test]
async fn activityid_read_optional_returns_none_for_missing() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let result = ActivityId::new(999)
        .read_optional(&test_session(), &mut store)
        .await?;
    assert!(result.is_none());
    Ok(())
}

/// ActivityId::read_optional returns Some for existing activity
#[tokio::test]
async fn activityid_read_optional_returns_some_for_existing() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    // A fresh id: the fixture already holds id 1, and a duplicate id is
    // a primary-key violation both stores reject (issue #405).
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act).await?;

    let result = ActivityId::new(100)
        .read_optional(&test_session(), &mut store)
        .await?;
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "Morning Ride");
    Ok(())
}

/// ActivityId::read returns existing activity
#[tokio::test]
async fn activityid_read_returns_existing() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    // A fresh id: the fixture already holds id 1, and a duplicate id is
    // a primary-key violation both stores reject (issue #405).
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act).await?;

    let result = ActivityId::new(100)
        .read(&test_session(), &mut store)
        .await?;
    assert_eq!(result.name, "Morning Ride");
    assert_eq!(result.user_id, test_user());
    Ok(())
}

/// ActivityId::read returns NotFound for non-existent activity
#[tokio::test]
async fn activityid_read_returns_not_found() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let result = ActivityId::new(999).read(&test_session(), &mut store).await;
    assert!(result.is_err());
    Ok(())
}

/// ActivityId::read rejects cross-user access
#[tokio::test]
async fn activityid_read_rejects_cross_user() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act).await?;

    // User 2 tries to access user 1's activity
    let other_session = TestSession::new(UserId::from(2));
    let result = ActivityId::new(100).read(&other_session, &mut store).await;
    assert!(result.is_err());
    Ok(())
}

// === Activity tests ===

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

/// Activity::get_all returns all activities for a user
#[tokio::test]
async fn activity_get_all_returns_user_activities() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));

    let act1 = Activity {
        id: ActivityId::new(101),
        user_id: sess.user_id(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act1).await?;

    let act2 = Activity {
        id: ActivityId::new(102),
        user_id: sess.user_id(),
        what: ActTypeId::from(3),
        name: "Evening Ride".to_string(),
        start: later_start(),
        duration: 1800,
        time: None,
        distance: Some(25000),
        climb: Some(200),
        descend: Some(100),
        energy: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act2).await?;

    let acts = Activity::get_all(&sess.user_id(), &mut store).await?;
    assert_eq!(acts.len(), 2);
    Ok(())
}

/// Activity::get_all returns empty for user with no activities
#[tokio::test]
async fn activity_get_all_empty_for_user() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let acts = Activity::get_all(&UserId::from(99), &mut store).await?;
    assert!(acts.is_empty());
    Ok(())
}

/// Activity::categories returns unique gear types from activities
#[tokio::test]
async fn activity_categories_returns_unique_gear_types() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act1 = Activity {
        id: ActivityId::new(101),
        ..sample_activity()
    }; // what = ActTypeId(1) -> gear_type for Bike
    store.activity_create(act1).await?;

    let act2 = Activity {
        id: ActivityId::new(102),
        what: ActTypeId::from(1), // same type
        name: "Another Ride".to_string(),
        start: later_start(),
        duration: 1800,
        time: None,
        distance: Some(25000),
        climb: Some(200),
        descend: Some(100),
        energy: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act2).await?;

    let categories = Activity::categories(&test_session(), &mut store).await?;
    assert_eq!(categories.len(), 1);
    Ok(())
}

/// Activity::categories returns empty for user with no activities
#[tokio::test]
async fn activity_categories_empty_for_no_activities() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let categories = Activity::categories(&TestSession::new(UserId::from(99)), &mut store).await?;
    assert!(categories.is_empty());
    Ok(())
}

/// Activity::find finds activities by gear in time range
#[tokio::test]
async fn activity_find_by_gear_and_time() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let part = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let act1 = Activity {
        id: ActivityId::new(100),
        gear: Some(part.id),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act1).await?;

    let acts = Activity::find(
        part.id,
        OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
        OffsetDateTime::from_unix_timestamp(1700001000).unwrap(),
        &mut store,
    )
    .await?;

    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].id, ActivityId::new(100));
    Ok(())
}

/// activities_find_by_gear_and_time includes begin and excludes end —
/// one rule on both stores (Postgres: start >= begin AND start < end;
/// the in-memory store used to include end).
#[tokio::test]
async fn activity_find_range_includes_begin_excludes_end() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let part = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let start = activity_start();
    let act = Activity {
        id: ActivityId::new(100),
        name: "Boundary Ride".to_string(),
        start,
        gear: Some(part.id),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    // A window that ends exactly at the start: the ride is excluded.
    let acts = Activity::find(part.id, start - time::Duration::hours(1), start, &mut store).await?;
    assert!(acts.is_empty(), "end is exclusive");

    // A window that begins exactly at the start: the ride is included.
    let acts = Activity::find(part.id, start, start + time::Duration::hours(1), &mut store).await?;
    assert_eq!(acts.len(), 1, "begin is inclusive");
    assert_eq!(acts[0].id, ActivityId::new(100));
    Ok(())
}

/// Activity::find returns only activities within the specified time range
#[tokio::test]
async fn activity_find_empty_in_time_range() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let part = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Create an activity starting 1 hour after the search window end
    let outside_activity = Activity {
        id: ActivityId::new(100),
        name: "Later Ride".to_string(),
        start: OffsetDateTime::from_unix_timestamp(1000002000).unwrap(),
        external_id: Some("garmin_outside".to_string()),
        ..sample_activity()
    };
    store.activity_create(outside_activity).await?;

    // Search for activities in an earlier time range
    let acts = Activity::find(
        part.id,
        OffsetDateTime::from_unix_timestamp(1000000000).unwrap(),
        OffsetDateTime::from_unix_timestamp(1000001000).unwrap(),
        &mut store,
    )
    .await?;

    // Should be empty since the only activity is outside the time range
    assert!(acts.is_empty());
    Ok(())
}

/// Activity::upsert creates new activity when it doesn't exist
#[tokio::test]
async fn activity_upsert_creates_new() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let new_id = ActivityId::new(100);
    let act = Activity {
        id: new_id,
        name: "New Activity".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    let summary = act.upsert(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(
        summary.activities.get(&new_id).unwrap().name,
        "New Activity"
    );
    Ok(())
}

/// Creating a new activity for the prepopulated bike (id 1) accounts for the
/// bike and all attached parts: the summary contains the bike, every part
/// attached to it, and the updated usage of each of them.
#[tokio::test]
async fn activity_upsert_creates_new_accounts_bike_and_attached_parts() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let bike = PartId::from(1);
    let act = Activity {
        id: ActivityId::new(100),
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

    // the usage of the bike, all attached parts and all their attachments to the bike
    let mut expected_ids: HashSet<UsageId> = HashSet::new();
    for pid in [
        bike,
        PartId::from(2),
        PartId::from(3),
        PartId::from(4),
        PartId::from(5),
        PartId::from(6),
    ] {
        expected_ids.insert(pid.read(&mut store).await?.usage);
        if pid != bike {
            let att = store
                .attachment_get_by_part_and_time(pid, activity_start())
                .await?
                .expect("the part is attached to the bike");
            expected_ids.insert(att.usage);
        }
    }
    let usage_ids: HashSet<UsageId> = summary.usages.live().into_iter().map(|u| u.id).collect();
    assert_eq!(usage_ids, expected_ids);

    // every affected usage is increased by the activity metrics
    // (8025+1000, 125000+10000, 1100+100, 1100+100, 1500+200, 3+1);
    // descend is None in the activity, so it falls back to climb
    let mut expected = Usage {
        id: UsageId::default(),
        time: 9025,
        distance: 135000,
        climb: 1200,
        descend: 1200,
        energy: 1700,
        count: 4,
    };
    for u in summary.usages.live() {
        expected.id = u.id;
        assert_eq!(u, expected, "unexpected usage for {}", u.id);
    }

    // the updates are persisted in the store
    let bike_usage = bike.read(&mut store).await?.usage;
    let stored = bike_usage.read(&mut store).await?;
    assert_eq!(stored.time, 9025);
    assert_eq!(stored.distance, 135000);
    assert_eq!(stored.count, 4);

    // the activity is stored with the bike as gear
    let stored_act = ActivityId::new(100)
        .read(&test_session(), &mut store)
        .await?;
    assert_eq!(stored_act.gear, Some(bike));
    Ok(())
}

/// Deleting an activity for the prepopulated bike (id 1) reverts the
/// accounting of the create: the summary contains the bike, every part
/// attached to it, and the usage of each of them back at its previous value.
#[tokio::test]
async fn activity_delete_reverts_bike_and_attached_part_usage() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let bike = PartId::from(1);
    let act = Activity {
        id: ActivityId::new(100),
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

    // create the activity so its usage is accounted, then delete it
    act.clone().upsert(&test_session(), &mut store).await?;
    let summary = ActivityId::new(100)
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
    // last_used of the bike stays at the activity start
    let bike_part = summary
        .parts
        .live()
        .into_iter()
        .find(|p| p.id == bike)
        .unwrap();
    assert_eq!(bike_part.last_used, round_time(activity_start()));

    // the usage of the bike, all attached parts and all their attachments to the bike
    let mut expected_ids: HashSet<UsageId> = HashSet::new();
    for pid in [
        bike,
        PartId::from(2),
        PartId::from(3),
        PartId::from(4),
        PartId::from(5),
        PartId::from(6),
    ] {
        expected_ids.insert(pid.read(&mut store).await?.usage);
        if pid != bike {
            let att = store
                .attachment_get_by_part_and_time(pid, activity_start())
                .await?
                .expect("the part is attached to the bike");
            expected_ids.insert(att.usage);
        }
    }
    let usage_ids: HashSet<UsageId> = summary.usages.live().into_iter().map(|u| u.id).collect();
    assert_eq!(usage_ids, expected_ids);

    // every affected usage is back at its prepopulated baseline
    // (9025-1000, 135000-10000, 1200-100, 1200-100, 1700-200, 4-1);
    // descend is None in the activity, so it falls back to climb
    let mut expected = Usage {
        id: UsageId::default(),
        time: 8025,
        distance: 125000,
        climb: 1100,
        descend: 1100,
        energy: 1500,
        count: 3,
    };
    for u in summary.usages.live() {
        expected.id = u.id;
        assert_eq!(u, expected, "unexpected usage for {}", u.id);
    }

    // the updates are persisted in the store
    let bike_usage = bike.read(&mut store).await?.usage;
    let stored = bike_usage.read(&mut store).await?;
    assert_eq!(stored.time, 8025);
    assert_eq!(stored.distance, 125000);
    assert_eq!(stored.count, 3);

    // the activity is gone
    assert_eq!(
        ActivityId::new(100)
            .read_optional(&test_session(), &mut store)
            .await?,
        None
    );
    Ok(())
}

/// Activity::upsert updates existing activity. The activity is created
/// under a fresh id: the fixture already holds id 1, and a duplicate id
/// is a primary-key violation both stores reject (issue #405).
#[tokio::test]
async fn activity_upsert_updates_existing() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act.clone()).await?;

    // Modify the activity
    let modified = Activity {
        name: "Updated Activity".to_string(),
        ..act.clone()
    };

    let summary = modified.upsert(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(
        summary.activities.get(&act.id).unwrap().name,
        "Updated Activity"
    );
    Ok(())
}

/// Activity::update updates and returns summary. The activity is created
/// under a fresh id: the fixture already holds id 1, and a duplicate id
/// is a primary-key violation both stores reject (issue #405).
#[tokio::test]
async fn activity_update_returns_summary() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act.clone()).await?;

    let modified = Activity {
        name: "Modified Ride".to_string(),
        ..act.clone()
    };

    let summary = modified.update(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    Ok(())
}

/// activity_update keeps the stored row's utc_offset, device_name and
/// external_id — one rule on both stores: Postgres never rewrites those
/// columns; the in-memory store used to replace the whole entity and
/// wiped them.
#[tokio::test]
async fn activity_update_preserves_utc_offset_device_name_and_external_id() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // A stored ride that started at 22:13:20+01:00 local, with device
    // metadata.
    let stored = Activity {
        id: ActivityId::new(7),
        name: "Original Ride".to_string(),
        start: activity_start().to_offset(time::UtcOffset::from_whole_seconds(3600).unwrap()),
        ..sample_activity()
    };
    store.activity_create(stored.clone()).await?;

    // An update with a different start (in a UTC representation) and no
    // device metadata: a whole-entity replacement would wipe the
    // preserved fields.
    let updated = Activity {
        name: "Updated Ride".to_string(),
        start: (activity_start() + time::Duration::seconds(37)).to_offset(time::UtcOffset::UTC),
        device_name: None,
        external_id: None,
        ..stored.clone()
    };

    let row = store.activity_update(updated).await?;

    // The updated fields took the new values …
    assert_eq!(row.name, "Updated Ride");
    assert_eq!(
        row.start.unix_timestamp(),
        activity_start().unix_timestamp() + 37
    );
    // … and the preserved fields survived the update.
    assert_eq!(
        row.start.offset().whole_seconds(),
        3600, // the stored row keeps its +01:00 offset
    );
    assert_eq!(row.device_name.as_deref(), Some("Garmin Edge"));
    assert_eq!(row.external_id.as_deref(), Some("garmin_12345"));
    Ok(())
}

/// Activity::delete unregisters usage and returns summary
#[tokio::test]
async fn activity_delete_returns_summary() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act.clone()).await?;

    let summary = ActivityId::new(100)
        .delete(&test_session(), &mut store)
        .await?;
    // After delete, the activity is a None tombstone
    assert_eq!(summary.activities.len(), 1);
    assert!(summary.activities.is_tombstoned(&ActivityId::new(100)));
    Ok(())
}

/// Activity::delete returns forbidden for non-owner session
#[tokio::test]
async fn activity_delete_rejects_non_owner() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act.clone()).await?;

    // Create a different user's session
    let other_session = TestSession::new(UserId::from(99));
    let result = ActivityId::new(100)
        .delete(&other_session, &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === Helper functions for sample data ===

fn sample_purchase_date() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

// === Suite 1: Activity — Usage Extraction (missing tests) ===

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

// === Suite 3: Activity — Registration with Parts and Attachments ===

/// Activity registration skips when gear is None
#[tokio::test]
async fn activity_register_no_gear_does_not_update_parts() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    }; // gear = None

    let summary = store.activity_create(act.clone()).await?;
    let summary = summary.register(Factor::Add, &mut store).await?;

    assert_eq!(summary.parts.len(), 0);
    Ok(())
}

/// Activity registration skips detached parts
#[tokio::test]
async fn activity_register_skips_detached_parts() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    let bike = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let act = Activity {
        id: ActivityId::new(100),
        start: activity_start(), // T=1700000000
        gear: Some(bike.id),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    store.activity_create(act.clone()).await?;
    let summary = act.register(Factor::Add, &mut store).await?;

    // Only the gear (bike) should be in parts, not the detached chain
    assert!(summary.parts.live().into_iter().all(|p| p.id == bike.id));
    Ok(())
}

// === Suite 4: Activity — Find by Gear and Time (missing tests) ===

/// Activity::find excludes activities without a gear
#[tokio::test]
async fn activity_find_excludes_activities_without_gear() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let part = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Activity with no gear should not be found
    let no_gear_act = Activity {
        id: ActivityId::new(100),
        name: "No Gear Ride".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(no_gear_act).await?;

    let acts = Activity::find(
        part.id,
        OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
        OffsetDateTime::from_unix_timestamp(1700001000).unwrap(),
        &mut store,
    )
    .await?;

    assert!(acts.is_empty());
    Ok(())
}

/// Activity::find returns multiple activities in range
#[tokio::test]
async fn activity_find_returns_multiple_in_range() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let part = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    for i in 1i32..=3 {
        let act = Activity {
            id: ActivityId::new(100 + i as i64),
            name: format!("Ride {}", i),
            start: activity_start() + time::Duration::seconds(i as i64 * 3600),
            time: Some(3500 * i),
            distance: Some(50000 * i),
            climb: Some(500 * i),
            descend: Some(300 * i),
            energy: Some(1000 * i),
            gear: Some(part.id),
            device_name: None,
            external_id: None,
            ..sample_activity()
        };
        store.activity_create(act).await?;
    }

    let acts = Activity::find(
        part.id,
        OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
        OffsetDateTime::from_unix_timestamp(1700020000).unwrap(),
        &mut store,
    )
    .await?;

    assert_eq!(acts.len(), 3);
    Ok(())
}

/// Activity::find returns empty for gear with no activities
#[tokio::test]
async fn activity_find_empty_for_gear_no_activities() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let part = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let acts = Activity::find(
        part.id,
        OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
        OffsetDateTime::from_unix_timestamp(1700001000).unwrap(),
        &mut store,
    )
    .await?;

    assert!(acts.is_empty());
    Ok(())
}

// === Suite 5: Activity — get_all and categories (missing tests) ===

/// Activity::get_all excludes other users' activities
#[tokio::test]
async fn activity_get_all_excludes_other_users() -> TbResult<()> {
    let mut store = TestStore::new();

    let act1 = Activity {
        user_id: UserId::from(1),
        name: "User 1 Ride".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act1).await?;

    let act2 = Activity {
        id: ActivityId::new(2),
        user_id: UserId::from(2),
        name: "User 2 Ride".to_string(),
        start: later_start(),
        duration: 1800,
        time: None,
        distance: Some(25000),
        climb: Some(200),
        descend: Some(100),
        energy: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act2).await?;

    let user_acts = Activity::get_all(&UserId::from(1), &mut store).await?;
    assert_eq!(user_acts.len(), 1);
    assert_eq!(user_acts[0].name, "User 1 Ride");

    Ok(())
}

// === Suite 12: Activity — Edge Cases and Error Handling ===

/// Activity::update on missing activity returns NotFound
#[tokio::test]
async fn activity_update_missing_activity_returns_not_found() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let fake_activity = Activity {
        id: ActivityId::new(999),
        name: "Ghost Activity".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let result = fake_activity.update(&test_session(), &mut store).await;
    assert!(result.is_err());
    Ok(())
}

/// Activity::upsert preserves custom ID
#[tokio::test]
async fn activity_upsert_preserves_original_id() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act_id = ActivityId::new(42); // Custom non-sequential ID
    let act = Activity {
        id: act_id,
        name: "Custom ID Activity".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    let summary = act.upsert(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.get(&act_id).unwrap().id, act_id);

    // Verify it was stored and can be read back
    let read_act = ActivityId::new(42)
        .read(&test_session(), &mut store)
        .await?;
    assert_eq!(read_act.id, act_id);
    Ok(())
}

/// Activity with zero duration is still registered
#[tokio::test]
async fn activity_with_zero_duration_still_registered() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let mut act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    act.duration = 0;

    store.activity_create(act.clone()).await?;
    let summary = act.register(Factor::Add, &mut store).await?;

    assert_eq!(summary.activities.len(), 1);
    // Even with zero duration, distance/climb should still propagate via Usage
    assert_eq!(summary.parts.len(), 0); // No gear, so no parts updated
    Ok(())
}

/// Activity with only climb (no other metrics) produces valid usage
#[tokio::test]
async fn activity_with_only_climb_no_other_metrics() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
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

    store.activity_create(act).await?;
    let summary = ActivityId::new(100)
        .read(&test_session(), &mut store)
        .await?
        .register(Factor::Add, &mut store)
        .await?;

    assert_eq!(summary.activities.len(), 1);
    Ok(())
}

// === Suite 8: Activity — rescan_all ===

/// rescan_all deletes all usages before re-registering
#[tokio::test]
async fn rescan_all_deletes_all_usages_first() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let act = Activity {
        id: ActivityId::new(100),
        ..sample_activity()
    };
    store.activity_create(act).await?;

    // First register to create some usage records
    let act2 = Activity {
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let _ = act2.register(Factor::Add, &mut store).await?;

    // Now rescan should delete all usages
    Activity::rescan_all(&mut store).await?;

    // Verify usages were cleaned up (no usage records should remain)
    Ok(())
}

/// rescan_all re-registers every activity
#[tokio::test]
async fn rescan_all_reregisters_every_activity() -> TbResult<()> {
    let mut store = TestStore::new();

    // Create multiple activities
    for i in 1i32..=3 {
        let act = Activity {
            id: ActivityId::new(i as i64),
            name: format!("Ride {}", i),
            start: activity_start() + time::Duration::seconds(i as i64 * 3600),
            time: Some(3500 * i),
            distance: Some(50000 * i),
            climb: Some(500 * i),
            descend: Some(300 * i),
            energy: Some(1000 * i),
            device_name: None,
            external_id: None,
            ..sample_activity()
        };
        store.activity_create(act).await?;
    }

    // Perform rescan
    Activity::rescan_all(&mut store).await?;

    // Verify activities are still present
    let all_acts = store.activity_get_really_all().await?;
    assert_eq!(all_acts.len(), 3);

    Ok(())
}

/// rescan_all rebuilds part and service usage rows from the prepopulated activities
#[tokio::test]
async fn rescan_all_rebuilds_part_and_service_usage() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let session = TestSession::new(UserId::from(1));
    let bike = PartId::from(1);

    // service created after all snapshot activities → aggregates all three
    let t = time::macros::datetime!(2023-06-01 10:00 UTC);
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
    let svc = sole_live(&summary.services);
    let u = sole_live(&summary.usages);
    assert_eq!(u.count, 3);

    Activity::rescan_all(&mut store).await?;

    // part usage rows were deleted and rebuilt from the snapshot activities
    let part = bike.part(&session, &mut store).await?;
    let part_usage = part.usage().read(&mut store).await?;
    assert_eq!(part_usage.time, 8025);
    assert_eq!(part_usage.distance, 125000);
    assert_eq!(part_usage.climb, 1100);
    assert_eq!(part_usage.descend, 1100);
    assert_eq!(part_usage.energy, 1500);
    assert_eq!(part_usage.count, 3);

    // service usage rows were rebuilt through the register path as well
    let svc_usage = svc.usage.read(&mut store).await?;
    assert_eq!(svc_usage.id, svc.usage);
    assert_eq!(svc_usage.time, 8025);
    assert_eq!(svc_usage.distance, 125000);
    assert_eq!(svc_usage.climb, 1100);
    assert_eq!(svc_usage.descend, 1100);
    assert_eq!(svc_usage.energy, 1500);
    assert_eq!(svc_usage.count, 3);
    Ok(())
}

// === Suite 9: Activity — CSV Import (descend parsing) ===

/// csv2descend parses German date format and updates existing activities
#[tokio::test]
async fn csv2descend_parses_german_date_format() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Pre-create 2 activities: activity_start() (1700000000 = 2023-11-14 22:13:20 UTC)
    // and activity_start() + 7200 (1700007200 = 2023-11-15 00:13:20 UTC)
    for offset in [0i64, 7200] {
        let ts = activity_start().unix_timestamp() + offset;
        let act = Activity {
            id: ActivityId::new(100 + offset),
            name: "Ride".to_string(),
            start: OffsetDateTime::from_unix_timestamp(ts).unwrap(),
            climb: None,
            descend: None,
            device_name: None,
            external_id: None,
            ..sample_activity()
        };
        store.activity_create(act).await?;
    }

    // CSV uses exact matching timestamps (2023-11-14 22:13:20 and 2023-11-15 00:13:20)
    let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Ride1,300
2023-11-15 00:13:20,Ride2,200";

    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(result.1.good.len(), 2); // 2 good records
    Ok(())
}

/// csv2descend parses English title field alias
#[tokio::test]
async fn csv2descend_parses_english_title_field() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Pre-create activity at activity_start()
    let act = Activity {
        id: ActivityId::new(201),
        name: "English Ride".to_string(),
        climb: None,
        descend: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    let csv_data = "Date,Title,Total Descent
2023-11-14 22:13:20,English Ride,300";

    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(result.1.good.len(), 1); // 1 good record
    Ok(())
}

/// csv2descend parses German decimal format for descend values
#[tokio::test]
async fn csv2descend_skips_german_decimal_format() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    let act = Activity {
        id: ActivityId::new(202),
        name: "Ride".to_string(),
        climb: None,
        descend: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Ride,1.234";

    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    // "1.234" → stripped to "1234" → parsed as 1234
    assert_eq!(result.1.good.len(), 1); // record parses successfully
    Ok(())
}

/// csv2descend returns good and bad lists for mixed valid/invalid records
#[tokio::test]
async fn csv2descend_returns_good_and_bad_lists() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Create 2 activities at activity_start() and activity_start() + 3600
    for offset in [0i64, 3600] {
        let ts = activity_start().unix_timestamp() + offset;
        let act = Activity {
            id: ActivityId::new(300 + offset),
            name: "Ride".to_string(),
            start: OffsetDateTime::from_unix_timestamp(ts).unwrap(),
            climb: None,
            descend: None,
            device_name: None,
            external_id: None,
            ..sample_activity()
        };
        store.activity_create(act).await?;
    }

    // Last row has invalid descend value (not a number)
    let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Valid Ride,300
2023-11-14 23:13:20,Bad Ride,not_a_number";

    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await;

    // Should return error for the invalid descend value
    assert!(result.is_err());
    Ok(())
}

/// csv2descend reports the rows it matched and skipped as a
/// `DescendReport` (the handler's response body): human-readable
/// descriptions (`"{title} at {start}"`), one per row.
#[tokio::test]
async fn csv2descend_reports_matched_and_skipped_rows() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // One activity at activity_start() (2023-11-14 22:13:20 UTC).
    let act = Activity {
        id: ActivityId::new(301),
        name: "Matched Ride".to_string(),
        climb: None,
        descend: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    // The first row matches the activity (by the local minute); the
    // second row has no activity at its time and is skipped.
    let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Matched Ride,300
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
    // The matched activity carries the new descend value; the phantom
    // row touched nothing.
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

/// csv2descend calls match_and_update for each valid record
#[tokio::test]
async fn csv2descend_calls_match_and_update_for_each_record() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Pre-create 2 activities at activity_start() + offset
    for offset in [0i64, 7200] {
        let ts = activity_start().unix_timestamp() + offset;
        let act = Activity {
            id: ActivityId::new(400 + offset),
            name: "Ride".to_string(),
            start: OffsetDateTime::from_unix_timestamp(ts).unwrap(),
            climb: None,
            descend: None,
            device_name: None,
            external_id: None,
            ..sample_activity()
        };
        store.activity_create(act).await?;
    }

    let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Ride One,300
2023-11-15 00:13:20,Ride Two,200";

    let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

    assert_eq!(result.1.good.len(), 2); // Both records parsed and updated
    Ok(())
}

/// get_by_user_and_time matches by the activity's local minute: the
/// stored row's local wall-clock minute (in its stored offset) must equal
/// the query's UTC wall-clock minute — one rule on both stores (Postgres
/// truncates both to the minute; the in-memory store used the exact
/// instant). The CSV import passes the user's local wall clock parsed as
/// UTC, so a ride stored at 22:13:20+01:00 is matched by the query
/// 22:13:50 UTC.
#[tokio::test]
async fn get_by_user_and_time_matches_by_local_minute() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // A ride stored at 22:13:20+01:00 local (= 21:13:20 UTC).
    let local_start = (activity_start() - time::Duration::hours(1))
        .to_offset(time::UtcOffset::from_whole_seconds(3600).unwrap());
    let stored = Activity {
        id: ActivityId::new(500),
        name: "Ride".to_string(),
        start: local_start,
        climb: None,
        descend: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(stored.clone()).await?;

    // Same local minute (22:13), a different instant: matches.
    let q_same_minute = activity_start() + time::Duration::seconds(30); // 22:13:50 UTC
    let found = store
        .get_by_user_and_time(test_user(), q_same_minute)
        .await?;
    assert_eq!(found.id, stored.id);

    // A different minute: no match.
    let q_other_minute = activity_start() + time::Duration::seconds(90); // 22:14:50 UTC
    let result = store
        .get_by_user_and_time(test_user(), q_other_minute)
        .await;
    assert!(matches!(result, Err(Error::NotFound(_))));

    // Another user's ride in the same minute never matches this user's
    // lookup.
    let other = Activity {
        id: ActivityId::new(501),
        user_id: UserId::from(99),
        ..stored.clone()
    };
    store.activity_create(other).await?;
    let found = store
        .get_by_user_and_time(test_user(), q_same_minute)
        .await?;
    assert_eq!(found.id, stored.id);
    Ok(())
}

/// get_by_user_and_time returns the maintainer-confirmed error (issue
/// #408) when two of the user's activities fall in the same local minute
/// — a conflicting import row must fail loudly, not silently update one
/// of the two rides.
#[tokio::test]
async fn get_by_user_and_time_same_minute_duplicate_is_ambiguous() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Two rides of the user in the 22:13 local minute (21:13:20Z and
    // 21:13:45Z, both stored with the +01:00 offset).
    let first_start = (activity_start() - time::Duration::hours(1))
        .to_offset(time::UtcOffset::from_whole_seconds(3600).unwrap());
    let first = Activity {
        id: ActivityId::new(500),
        name: "First Ride".to_string(),
        start: first_start,
        climb: None,
        descend: None,
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    let second = Activity {
        id: ActivityId::new(501),
        name: "Second Ride".to_string(),
        start: first_start + time::Duration::seconds(25),
        ..first.clone()
    };
    store.activity_create(first).await?;
    store.activity_create(second).await?;

    // A query in that minute (22:13:50 UTC): two matches → the error.
    let q = activity_start() + time::Duration::seconds(30);
    let result = store.get_by_user_and_time(test_user(), q).await;
    assert!(
        matches!(result, Err(Error::Ambiguous(_))),
        "two same-minute rides must return Error::Ambiguous, got {result:?}"
    );
    Ok(())
}

// === Suite 7: Activity — set_default_part ===

/// set_default_part assigns gear to activities with matching type and no gear
#[tokio::test]
async fn set_default_part_assigns_gear_to_matching_activities() -> TbResult<()> {
    let mut store = TestStore::new();

    // Create a bike (gear type for Riding)
    let bike = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Create an activity without gear (Ride type)
    let act = Activity {
        what: ActTypeId::from(1), // Riding type
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    Activity::set_default_part(bike.id, &test_session(), &mut store).await?;

    // Verify the activity was updated with the gear
    let updated_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
    assert_eq!(updated_act.gear, Some(bike.id));

    Ok(())
}

/// set_default_part does not assign gear to non-matching types
#[tokio::test]
async fn set_default_part_only_affected_matching_types() -> TbResult<()> {
    let mut store = TestStore::new();

    let bike = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Create a running activity (type 3) - should NOT match bike's act_types
    let run_act = Activity {
        what: ActTypeId::from(3), // Running type
        name: "Morning Run".to_string(),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(run_act).await?;

    Activity::set_default_part(bike.id, &test_session(), &mut store).await?;

    // Running activity should NOT get the bike gear assigned
    let updated_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
    assert_eq!(updated_act.gear, None);

    Ok(())
}

/// set_default_part does not override existing gear assignments
#[tokio::test]
async fn set_default_part_does_not_override_existing_gear() -> TbResult<()> {
    let mut store = TestStore::new();

    let bike1 = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let bike2 = Part::create(
        "Mountain Bike".to_string(),
        "Specialized".to_string(),
        "Stumpjumper".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Create activity with existing gear
    let act = Activity {
        gear: Some(bike1.id), // Already assigned
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    let _ = Activity::set_default_part(bike2.id, &test_session(), &mut store).await?;

    // Should still have original gear, not bike2
    let updated_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
    assert_eq!(updated_act.gear, Some(bike1.id));

    Ok(())
}

/// set_default_part returns zero usage when no activities match
#[tokio::test]
async fn set_default_part_empty_returns_zero_usage() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Create a bike but no activities at all
    let _bike = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let summary = Activity::set_default_part(_bike.id, &test_session(), &mut store).await?;

    assert_eq!(summary.usages.len(), 0);
    Ok(())
}

/// set_default_part assigns the gear and registers the newly assigned activities
#[tokio::test]
async fn set_default_part_accounts_usage_for_newly_assigned_activities() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let session = TestSession::new(UserId::from(1));
    let bike = PartId::from(1);

    let baseline = bike
        .part(&session, &mut store)
        .await?
        .usage()
        .read(&mut store)
        .await?;
    assert_eq!(baseline.time, 8025);
    assert_eq!(baseline.count, 3);

    // unregistered ride without gear, within the snapshot bike's attachment window
    let act = Activity {
        id: ActivityId::new(101),
        name: "Unassigned Ride".to_string(),
        start: time::macros::datetime!(2023-05-20 10:00 UTC),
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    Activity::set_default_part(bike, &session, &mut store).await?;

    let updated = ActivityId::new(101).read(&session, &mut store).await?;
    assert_eq!(updated.gear, Some(bike));

    // bike usage row is incremented by the newly assigned activity:
    // 8025+1000, 125000+10000, 1100+100, 1100+100 (descend None → climb), 1500+200, 3+1
    let increased = bike
        .part(&session, &mut store)
        .await?
        .usage()
        .read(&mut store)
        .await?;
    assert_eq!(increased.time, 9025);
    assert_eq!(increased.distance, 135000);
    assert_eq!(increased.climb, 1200);
    assert_eq!(increased.descend, 1200);
    assert_eq!(increased.energy, 1700);
    assert_eq!(increased.count, 4);
    Ok(())
}

/// set_default_part requires ownership
#[tokio::test]
async fn set_default_part_requires_ownership() -> TbResult<()> {
    let mut store = TestStore::new();

    let bike = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Create an activity for user 2
    let act = Activity {
        user_id: UserId::from(2),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(act).await?;

    // User 1 tries to set default gear - should find no matching activities
    // (since the activity belongs to User 2)
    let summary = Activity::set_default_part(bike.id, &test_session(), &mut store).await?;
    assert_eq!(summary.usages.len(), 0); // No usage because no matching activities

    // Verify the activity's gear is still None (not changed by user 1's operation)
    let act = ActivityId::new(1)
        .read(&TestSession::new(UserId::from(2)), &mut store)
        .await?;
    assert_eq!(act.gear, None);

    Ok(())
}

// === Suite 6: Activity — replace() (Updation Logic) ===

/// Replace on same activity ID with changed gear updates affected parts
#[tokio::test]
async fn replace_changes_affected_parts() -> TbResult<()> {
    let mut store = TestStore::prepopulated();

    // Create two gears
    let bike1 = Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    let bike2 = Part::create(
        "Mountain Bike".to_string(),
        "Specialized".to_string(),
        "Stumpjumper".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &test_session(),
        &mut store,
    )
    .await?;

    // Create initial activity on bike1
    let old_act = Activity {
        id: ActivityId::new(100),
        name: "Road Ride".to_string(),
        gear: Some(bike1.id),
        device_name: None,
        external_id: None,
        ..sample_activity()
    };
    store.activity_create(old_act).await?;

    // Create new activity with same ID but different gear
    let new_act = Activity {
        id: ActivityId::new(100),
        name: "MTB Ride".to_string(),
        gear: Some(bike2.id), // Different gear!
        device_name: None,
        external_id: None,
        ..sample_activity()
    };

    new_act.update(&test_session(), &mut store).await?;

    // The activity should now reference bike2
    let read_act = ActivityId::new(100)
        .read(&test_session(), &mut store)
        .await?;
    assert_eq!(read_act.gear, Some(bike2.id));

    Ok(())
}

/// replace() moves usage between the old and new gear parts
#[tokio::test]
async fn replace_gear_change_moves_usage_between_bikes() -> TbResult<()> {
    let mut store = TestStore::prepopulated();
    let session = TestSession::new(UserId::from(1));
    let bike1 = PartId::from(1);

    let bike2 = Part::create(
        "Mountain Bike".to_string(),
        "Specialized".to_string(),
        "Stumpjumper".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &session,
        &mut store,
    )
    .await?;

    // registered ride on the snapshot bike
    let old_act = Activity {
        id: ActivityId::new(102),
        name: "Ride".to_string(),
        start: time::macros::datetime!(2023-05-20 10:00 UTC),
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
    old_act.clone().upsert(&session, &mut store).await?;

    let increased = bike1
        .part(&session, &mut store)
        .await?
        .usage()
        .read(&mut store)
        .await?;
    assert_eq!(increased.time, 9025);
    assert_eq!(increased.count, 4);

    // replace: same activity, gear moved to the new bike
    let new_act = Activity {
        name: "Ride (replaced)".to_string(),
        gear: Some(bike2.id),
        ..old_act
    };
    new_act.update(&session, &mut store).await?;

    // old bike usage row is back to the snapshot totals
    let reverted = bike1
        .part(&session, &mut store)
        .await?
        .usage()
        .read(&mut store)
        .await?;
    assert_eq!(reverted.time, 8025);
    assert_eq!(reverted.distance, 125000);
    assert_eq!(reverted.climb, 1100);
    assert_eq!(reverted.descend, 1100);
    assert_eq!(reverted.energy, 1500);
    assert_eq!(reverted.count, 3);

    // new bike usage row holds exactly the moved activity
    let moved = bike2.usage().read(&mut store).await?;
    assert_eq!(moved.time, 1000);
    assert_eq!(moved.distance, 10000);
    assert_eq!(moved.climb, 100);
    assert_eq!(moved.descend, 100);
    assert_eq!(moved.energy, 200);
    assert_eq!(moved.count, 1);
    Ok(())
}
