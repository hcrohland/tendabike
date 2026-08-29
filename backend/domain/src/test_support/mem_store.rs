//! Prepopulated MemStore for tests.
//!
//! Provides a `MemStore::prepopulated()` that returns a store with meaningful
//! test data: 2 bikes with wheels and tires, spares including one assembled
//! wheel, plus sample activities for usage calculation testing.

use super::MemStore;
use super::fixtures::{sample_purchase_date, test_session};
use super::part_type_ids::{BIKE, CHAIN, FRONT_WHEEL, REAR_WHEEL, TIRE};
use crate::traits::{ActivityStore, AttachmentStore};
use crate::{
    ActTypeId, Activity, Attachment, OffsetDateTime, Part, PartId, PartTypeId, TbResult, Usage,
};

impl MemStore {
    /// Returns a store prepopulated with workshop data.
    pub fn prepopulated() -> Self {
        let snap: StoreSnapshot = serde_json::from_str(super::prepopulated_data::SNAPSHOT_JSON)
            .expect("SNAPSHOT_JSON must be valid JSON");

        let mut store = Self::new();

        for part in snap.parts {
            store.parts.insert(part.id, part);
        }

        for att in snap.attachments {
            let key = (att.part_id, att.attached, 0);
            store.attachments.insert(key, att);
        }

        for usage in snap.usages {
            store.usages.insert(usage.id, usage);
        }

        for activity in snap.activities {
            store.activities.push(activity);
        }

        store
    }
}

const ATTACH_TIME: OffsetDateTime = time::macros::datetime!(2023-01-01 00:00 UTC);

/// Serializable snapshot of store data for JSON persistence.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoreSnapshot {
    pub parts: Vec<Part>,
    pub attachments: Vec<Attachment>,
    pub usages: Vec<Usage>,
    pub activities: Vec<Activity>,
}

impl MemStore {
    /// Export store data as a serializable snapshot.
    pub fn snapshot(&self) -> StoreSnapshot {
        StoreSnapshot {
            parts: self.parts.values().cloned().collect(),
            attachments: self.attachments.values().cloned().collect(),
            usages: self.usages.values().cloned().collect(),
            activities: self.activities.clone(),
        }
    }
}

async fn build_store() -> TbResult<MemStore> {
    let mut store = MemStore::new();
    let s = test_session();

    // ─── Bike A: Main Bike ────────────────────────────────────────────
    let bike_a = Part::create(
        "Main Bike".into(),
        "TendaBike".into(),
        "Standard Frame".into(),
        BIKE,
        None,
        sample_purchase_date() - time::Duration::days(365),
        "Main bike frame".into(),
        &s,
        &mut store,
    )
    .await?;

    let fw_a = mkpart(
        "Front Wheel A",
        "Zipp",
        "404 Firecrest",
        FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let rw_a = mkpart(
        "Rear Wheel A",
        "DT Swiss",
        "XR 1501",
        REAR_WHEEL,
        &mut store,
    )
    .await?;
    let ch_a = mkpart("Chain A", "Shimano", "CN-M510", CHAIN, &mut store).await?;
    let t_a1 = mkpart("Tire Front A", "Continental", "GP5000", TIRE, &mut store).await?;
    let t_a2 = mkpart("Tire Rear A", "Continental", "GP5000 ST", TIRE, &mut store).await?;

    do_subpart(&mut store, fw_a.id, bike_a.id, BIKE).await?;
    do_subpart(&mut store, rw_a.id, bike_a.id, BIKE).await?;
    do_subpart(&mut store, ch_a.id, bike_a.id, BIKE).await?;
    do_subpart(&mut store, t_a1.id, fw_a.id, TIRE).await?;
    do_subpart(&mut store, t_a2.id, fw_a.id, TIRE).await?;
    do_subpart(&mut store, t_a2.id, rw_a.id, TIRE).await?;

    // ─── Bike B: Road Bike ────────────────────────────────────────────
    let bike_b = Part::create(
        "Road Bike".into(),
        "Trek".into(),
        "Domane".into(),
        BIKE,
        None,
        sample_purchase_date() - time::Duration::days(365),
        "Road bike frame".into(),
        &s,
        &mut store,
    )
    .await?;

    let fw_b = mkpart(
        "Front Wheel B",
        "Mavic",
        "Carbon WS",
        FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let rw_b = mkpart("Rear Wheel B", "Mavic", "Carbon WS", REAR_WHEEL, &mut store).await?;
    let ch_b = mkpart("Chain B", "SRAM", "PC-XX1", CHAIN, &mut store).await?;
    let t_b1 = mkpart("Tire Front B", "Schwalbe", "One", TIRE, &mut store).await?;
    let t_b2 = mkpart("Tire Rear B", "Schwalbe", "One Plus", TIRE, &mut store).await?;

    do_subpart(&mut store, fw_b.id, bike_b.id, BIKE).await?;
    do_subpart(&mut store, rw_b.id, bike_b.id, BIKE).await?;
    do_subpart(&mut store, ch_b.id, bike_b.id, BIKE).await?;
    do_subpart(&mut store, t_b1.id, fw_b.id, TIRE).await?;
    do_subpart(&mut store, t_b2.id, fw_b.id, TIRE).await?;
    do_subpart(&mut store, t_b2.id, rw_b.id, TIRE).await?;

    // ─── Spares (4 loose + 1 assembled) ──────────────────────────────
    let _sc1 = mkpart("Spare Chain 1", "Shimano", "HG-54", CHAIN, &mut store).await?;
    let _sc2 = mkpart("Spare Chain 2", "SRAM", "PC-1031", CHAIN, &mut store).await?;
    let _st = mkpart("Spare Tire", "Continental", "Supersonic", TIRE, &mut store).await?;

    let sw = mkpart("Spare Wheel", "HED", "Stinger 3", FRONT_WHEEL, &mut store).await?;
    let st2 = mkpart(
        "Spare Wheel Tire",
        "Continental",
        "GP5000",
        TIRE,
        &mut store,
    )
    .await?;
    do_subpart(&mut store, sw.id, bike_a.id, BIKE).await?;
    do_subpart(&mut store, st2.id, sw.id, TIRE).await?;

    // ─── Activities on Bike A (for usage calculation) ─────────────────
    let base = sample_purchase_date() - time::Duration::days(180);
    store
        .activity_create(mk_activity(
            "Morning Ride",
            base,
            3600,
            Some(25),
            Some(50_000),
            Some(400),
            bike_a.id,
        ))
        .await?;
    store
        .activity_create(mk_activity(
            "Hill Repeats",
            base + time::Duration::hours(2),
            1800,
            Some(5200),
            Some(40_000),
            Some(600),
            bike_a.id,
        ))
        .await?;
    store
        .activity_create(mk_activity(
            "Recovery Spin",
            base + time::Duration::days(1),
            2400,
            Some(2800),
            Some(35_000),
            Some(100),
            bike_a.id,
        ))
        .await?;

    // Compute usage from activities — populates usages for Bike A + all attached subparts
    Activity::rescan_all(&mut store).await?;

    Ok(store)
}

async fn mkpart(
    name: &str,
    vendor: &str,
    model: &str,
    typ: PartTypeId,
    store: &mut MemStore,
) -> TbResult<Part> {
    let s = test_session();
    Part::create(
        name.into(),
        vendor.into(),
        model.into(),
        typ,
        None,
        sample_purchase_date(),
        "Test part".into(),
        &s,
        store,
    )
    .await
}

async fn do_subpart(
    store: &mut MemStore,
    part_id: PartId,
    gear_id: PartId,
    hook: PartTypeId,
) -> TbResult<()> {
    let att = Attachment::new(part_id, ATTACH_TIME, gear_id, hook, crate::MAX_TIME);
    store.attachment_create(att).await?;
    Ok(())
}

fn mk_activity(
    name: &str,
    start: OffsetDateTime,
    duration: i32,
    time: Option<i32>,
    distance: Option<i32>,
    climb: Option<i32>,
    gear: PartId,
) -> Activity {
    Activity {
        id: crate::ActivityId::new(1),
        user_id: crate::UserId::from(1),
        what: ActTypeId::from(1),
        name: name.into(),
        start,
        duration,
        time,
        distance,
        climb,
        descend: None,
        energy: Some(500),
        gear: Some(gear),
        device_name: None,
        external_id: None,
    }
}

/// Builds a prepopulated store and writes formatted snapshot to the data file.
/// Run: `cargo test -p tb_domain build_snapshot_json -- --nocapture`
#[tokio::test]
async fn build_snapshot_json() {
    let store = build_store().await.unwrap();

    // Count active usages for verification
    let active_count = store
        .usages
        .values()
        .filter(|u| u.time > 0 || u.distance > 0)
        .count();
    eprintln!("Active usages: {}", active_count);

    // Generate pretty-printed JSON
    let json = serde_json::to_string_pretty(&store.snapshot()).unwrap();

    // Write to prepopulated_data.rs as a raw string constant
    let rs_content = format!(
        "//! Prepopulated store data for tests.\n\
         //! Generated from `build_snapshot_json` test.\n\
         //! Regenerate by running: cargo test -p tb_domain build_snapshot_json -- --nocapture\n\
         \n\
         /// Formatted JSON snapshot of the prepopulated store.\n\
         /// This is a pretty-printed JSON string that gets deserialized by MemStore::prepopulated().\n\
         pub const SNAPSHOT_JSON: &str = r#\"{}\"#;\n",
        json
    );

    std::fs::write("src/test_support/prepopulated_data.rs", &rs_content).unwrap();
    eprintln!("→ Wrote prepopulated_data.rs");
}
