//! The canonical fixture path (moved from `tb_domain::test_support`,
//! spec #510): the `StoreSnapshot` type, the `prepopulated()` loader,
//! the `snapshot()` export, and `build_workshop_store()` — the generator
//! for the JSON snapshot that the `build-snapshot` bin
//! (`cargo run -p tb_teststore --bin build-snapshot`) rebuilds
//! `prepopulated_data.rs` from.
//!
//! Note: usage UUIDs are generated via `Uuid::now()` (v7, wall-clock) and
//! are NOT deterministic across regenerations. The snapshot structure
//! (parts, attachments, users, activities) is deterministic; only the
//! UUID values differ between builds.

use super::{TestStore, TestStoreState};
use crate::fixtures::{sample_purchase_date, test_session};
use crate::prepopulated_data;
use crate::store_data::StoreData;
use tb_domain::test_support::part_type_ids::{BIKE, CHAIN, FRONT_WHEEL, REAR_WHEEL, TIRE};
use tb_domain::{
    ActTypeId, Activity, Attachment, Part, PartId, PartTypeId, Session, TbResult, Usage, User,
    UserStore, attach_assembly,
};
use time::OffsetDateTime;

/// Serializable snapshot of store data for JSON persistence.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoreSnapshot {
    pub users: Vec<User>,
    pub parts: Vec<Part>,
    pub attachments: Vec<Attachment>,
    pub usages: Vec<Usage>,
    pub activities: Vec<Activity>,
}

impl TestStore {
    /// Returns a store prepopulated with workshop data: the
    /// `StoreSnapshot` is loaded into a fresh state, id counters derived
    /// from the loaded data (spec #510).
    pub fn prepopulated() -> Self {
        let snap: StoreSnapshot = serde_json::from_str(prepopulated_data::SNAPSHOT_JSON)
            .expect("SNAPSHOT_JSON must be valid JSON");

        let mut data = StoreData::empty();

        let max_part_id: i32 = if !snap.parts.is_empty() {
            snap.parts.iter().map(|p| p.id.into()).max().unwrap_or(0)
        } else {
            0
        };
        let max_user_id: i32 = if !snap.users.is_empty() {
            snap.users.iter().map(|u| u.id.into()).max().unwrap_or(0)
        } else {
            0
        };
        let max_shop_id: i32 = if !snap.parts.iter().any(|p| p.shop.is_some()) {
            0
        } else {
            snap.parts
                .iter()
                .filter_map(|p| p.shop.map(|s| s.into()))
                .max()
                .unwrap_or(0)
        };

        for user in snap.users {
            data.users.insert(user.id, user);
        }

        for part in snap.parts {
            data.parts.insert(part.id, part);
        }

        for att in &snap.attachments {
            data.attachments.insert((att.part_id, att.attached), *att);
        }

        for usage in snap.usages {
            data.usages.insert(usage.id, usage);
        }

        for activity in snap.activities {
            data.activities.push(activity);
        }

        data.next_part_id = max_part_id + 1;
        data.next_user_id = max_user_id + 1;
        data.next_shop_id = max_shop_id + 1;
        data.next_subscription_id = 1;

        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(TestStoreState {
                domain: data,
                strava: crate::StravaState::default(),
            })),
            panic: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Export store data as a serializable snapshot, deterministically
    /// ordered (spec #510).
    pub fn snapshot(&self) -> StoreSnapshot {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        let mut snap = StoreSnapshot {
            users: d.users.values().cloned().collect(),
            parts: d.parts.values().cloned().collect(),
            attachments: d.attachments.values().cloned().collect(),
            usages: d.usages.values().cloned().collect(),
            activities: d.activities.clone(),
        };
        snap.users.sort_by_key(|u| i32::from(u.id));
        snap.parts.sort_by_key(|p| p.id);
        snap.attachments.sort_by_key(|a| (a.part_id, a.attached));
        snap.usages.sort_by_key(|u| u.id);
        snap.activities.sort_by_key(|a| a.id);
        snap
    }
}

const ATTACH_TIME: OffsetDateTime = time::macros::datetime!(2023-01-01 00:00 UTC);

/// Builds the canonical workshop store used as the source of
/// `SNAPSHOT_JSON` (moved from `tb_domain::test_support`, spec #510).
pub async fn build_workshop_store() -> TbResult<TestStore> {
    let mut store = TestStore::new();
    let s = test_session();

    // Create user 1 (the customer who owns all workshop parts)
    store.create("Tenda", "Bike", &None).await?;

    // ─── Bike A: Main Bike ────────────────────────────────────────────
    let bike_a = Part::create(
        "Main Bike".into(),
        "TendaBike".into(),
        "Standard Frame".into(),
        BIKE,
        None,
        sample_purchase_date() - time::Duration::days(365),
        &s,
        &mut store,
    )
    .await?;

    let fw_a = mk_part(
        &s,
        "Front Wheel A",
        "Zipp",
        "404 Firecrest",
        FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let rw_a = mk_part(
        &s,
        "Rear Wheel A",
        "DT Swiss",
        "XR 1501",
        REAR_WHEEL,
        &mut store,
    )
    .await?;
    let ch_a = mk_part(&s, "Chain A", "Shimano", "CN-M510", CHAIN, &mut store).await?;
    let t_a1 = mk_part(
        &s,
        "Tire Front A",
        "Continental",
        "GP5000",
        TIRE,
        &mut store,
    )
    .await?;
    let t_a2 = mk_part(
        &s,
        "Tire Rear A",
        "Continental",
        "GP5000 ST",
        TIRE,
        &mut store,
    )
    .await?;

    attach_assembly(&s, fw_a.id, ATTACH_TIME, bike_a.id, BIKE, false, &mut store).await?;
    attach_assembly(&s, rw_a.id, ATTACH_TIME, bike_a.id, BIKE, false, &mut store).await?;
    attach_assembly(&s, ch_a.id, ATTACH_TIME, bike_a.id, BIKE, false, &mut store).await?;
    // tires on mounted wheels carry the bike as gear (flat row model, ADR-0003):
    // the hook records the wheel, the gear is the top-level part
    attach_assembly(
        &s,
        t_a1.id,
        ATTACH_TIME,
        bike_a.id,
        FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        t_a2.id,
        ATTACH_TIME,
        bike_a.id,
        REAR_WHEEL,
        false,
        &mut store,
    )
    .await?;

    // ─── Bike B: Road Bike ────────────────────────────────────────────
    let bike_b = Part::create(
        "Road Bike".into(),
        "Trek".into(),
        "Domane".into(),
        BIKE,
        None,
        sample_purchase_date() - time::Duration::days(365),
        &s,
        &mut store,
    )
    .await?;

    let fw_b = mk_part(
        &s,
        "Front Wheel B",
        "Mavic",
        "Carbon WS",
        FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let rw_b = mk_part(
        &s,
        "Rear Wheel B",
        "Mavic",
        "Carbon WS",
        REAR_WHEEL,
        &mut store,
    )
    .await?;
    let ch_b = mk_part(&s, "Chain B", "SRAM", "PC-XX1", CHAIN, &mut store).await?;
    let t_b1 = mk_part(&s, "Tire Front B", "Schwalbe", "One", TIRE, &mut store).await?;
    let t_b2 = mk_part(&s, "Tire Rear B", "Schwalbe", "One Plus", TIRE, &mut store).await?;

    attach_assembly(&s, fw_b.id, ATTACH_TIME, bike_b.id, BIKE, false, &mut store).await?;
    attach_assembly(&s, rw_b.id, ATTACH_TIME, bike_b.id, BIKE, false, &mut store).await?;
    attach_assembly(&s, ch_b.id, ATTACH_TIME, bike_b.id, BIKE, false, &mut store).await?;
    // same as Bike A: tires on mounted wheels carry the bike as gear
    attach_assembly(
        &s,
        t_b1.id,
        ATTACH_TIME,
        bike_b.id,
        FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        t_b2.id,
        ATTACH_TIME,
        bike_b.id,
        REAR_WHEEL,
        false,
        &mut store,
    )
    .await?;

    // ─── Spares (loose; the spare wheel is not mounted, just carries its tire) ───
    let _sc1 = mk_part(&s, "Spare Chain 1", "Shimano", "HG-54", CHAIN, &mut store).await?;
    let _sc2 = mk_part(&s, "Spare Chain 2", "SRAM", "PC-1031", CHAIN, &mut store).await?;
    let _st = mk_part(
        &s,
        "Spare Tire",
        "Continental",
        "Supersonic",
        TIRE,
        &mut store,
    )
    .await?;

    let sw = mk_part(
        &s,
        "Spare Wheel",
        "HED",
        "Stinger 3",
        FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let st2 = mk_part(
        &s,
        "Spare Wheel Tire",
        "Continental",
        "GP5000",
        TIRE,
        &mut store,
    )
    .await?;
    // the spare wheel is a loose spare (not mounted on a bike) but carries its tire
    attach_assembly(
        &s,
        st2.id,
        ATTACH_TIME,
        sw.id,
        FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;

    // ─── Activities on Bike A (for usage calculation) ─────────────────
    let base = sample_purchase_date() - time::Duration::days(180);
    mk_activity(
        tb_domain::ActivityId::new(1),
        "Morning Ride",
        base,
        3600,
        Some(25),
        Some(50_000),
        Some(400),
        bike_a.id,
    )
    .upsert(&s, &mut store)
    .await?;
    mk_activity(
        tb_domain::ActivityId::new(2),
        "Hill Repeats",
        base + time::Duration::hours(2),
        1800,
        Some(5200),
        Some(40_000),
        Some(600),
        bike_a.id,
    )
    .upsert(&s, &mut store)
    .await?;
    mk_activity(
        tb_domain::ActivityId::new(3),
        "Recovery Spin",
        base + time::Duration::days(1),
        2400,
        Some(2800),
        Some(35_000),
        Some(100),
        bike_a.id,
    )
    .upsert(&s, &mut store)
    .await?;

    // Compute usage from activities — populates usages for Bike A + all attached subparts
    Activity::rescan_all(&mut store).await?;

    Ok(store)
}

async fn mk_part(
    session: &dyn Session,
    name: &str,
    vendor: &str,
    model: &str,
    typ: PartTypeId,
    store: &mut TestStore,
) -> TbResult<Part> {
    Part::create(
        name.into(),
        vendor.into(),
        model.into(),
        typ,
        None,
        sample_purchase_date(),
        session,
        store,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
fn mk_activity(
    id: tb_domain::ActivityId,
    name: &str,
    start: OffsetDateTime,
    duration: i32,
    time: Option<i32>,
    distance: Option<i32>,
    climb: Option<i32>,
    gear: PartId,
) -> Activity {
    Activity {
        id,
        user_id: tb_domain::UserId::from(1),
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
