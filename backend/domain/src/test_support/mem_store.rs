//! Prepopulated MemStore for tests.
//!
//! Provides a `MemStore::prepopulated()` that returns a store with meaningful
//! test data: 2 bikes with wheels and tires, spares including one assembled
//! wheel, plus sample activities for usage calculation testing.
//!
//! `build_workshop_store()` is the generator for the JSON snapshot; the
//! `build-snapshot` bin (`cargo run -p tb_domain --bin build-snapshot
//! --features test-support`) rebuilds `prepopulated_data.rs` from it.
//!
//! Note: usage UUIDs are generated via `Uuid::now()` (v7, wall-clock) and are
//! NOT deterministic across regenerations. The snapshot structure (parts,
//! attachments, users, activities) is deterministic; only the UUID values
//! differ between builds.

use super::MemStore;
use super::fixtures::{sample_purchase_date, test_session};
use super::part_type_ids::{BIKE, CHAIN, FRONT_WHEEL, REAR_WHEEL, TIRE};
use crate::traits::UserStore;
use crate::{
    ActTypeId, Activity, Attachment, OffsetDateTime, Part, PartId, PartTypeId, Session, TbResult,
    Usage, User, attach_assembly,
};

impl MemStore {
    /// Returns a store prepopulated with workshop data.
    pub fn prepopulated() -> Self {
        let snap: StoreSnapshot = serde_json::from_str(super::prepopulated_data::SNAPSHOT_JSON)
            .expect("SNAPSHOT_JSON must be valid JSON");

        let store = Self::new();
        let mut data = super::StoreData::empty();

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

        // Seed the committed database with the workshop data; the store's
        // transaction starts clean on top of it.
        *store.base.write().expect("in-memory store base poisoned") = data;
        store
    }
}

const ATTACH_TIME: OffsetDateTime = time::macros::datetime!(2023-01-01 00:00 UTC);

/// Serializable snapshot of store data for JSON persistence.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoreSnapshot {
    pub users: Vec<User>,
    pub parts: Vec<Part>,
    pub attachments: Vec<Attachment>,
    pub usages: Vec<Usage>,
    pub activities: Vec<Activity>,
}

impl MemStore {
    /// Export store data as a serializable snapshot, deterministically ordered.
    pub fn snapshot(&self) -> StoreSnapshot {
        let d = self.state();
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

/// Builds the canonical workshop store used as the source of `SNAPSHOT_JSON`.
pub async fn build_workshop_store() -> TbResult<MemStore> {
    let mut store = MemStore::new();
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
        crate::ActivityId::new(1),
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
        crate::ActivityId::new(2),
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
        crate::ActivityId::new(3),
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
    store: &mut MemStore,
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
    id: crate::ActivityId,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fault;
    use crate::test_support::fixtures::{fixture_bike, test_session};
    use crate::test_support::part_type_ids::FRONT_WHEEL;
    use crate::traits::UserStore;
    use crate::{Attachment, Error, Part, UserId};
    use time::Duration;

    const T: time::OffsetDateTime = time::macros::datetime!(2024-01-15 12:00 UTC);

    /// Commit lands writes in the database: a sibling transaction that began
    /// before the write cannot see it until the write is committed, and can
    /// see it afterwards (issue #409).
    #[tokio::test]
    async fn commit_lands_writes_in_the_database() -> TbResult<()> {
        let mut store = MemStore::new();
        let mut sibling = store.begin();

        store.create("Ada", "Lovelace", &None).await?;

        // not committed yet: invisible to the sibling transaction
        assert!(sibling.get(UserId::from(1)).await.is_err());

        store.commit().await?;

        let user = sibling.get(UserId::from(1)).await?;
        assert_eq!(user.firstname, "Ada");
        Ok(())
    }

    /// An aborted (dropped) store discards its writes, like the database
    /// would roll back an uncommitted transaction (issue #409).
    #[tokio::test]
    async fn aborted_store_discards_writes() -> TbResult<()> {
        let mut store = MemStore::new();
        let mut sibling = store.begin();

        store.create("Ada", "Lovelace", &None).await?;
        drop(store); // the transaction is aborted without commit

        assert!(sibling.get(UserId::from(1)).await.is_err());
        Ok(())
    }

    /// `rollback()` discards everything this transaction has written: the
    /// store is back at exactly the committed state (issue #409).
    #[tokio::test]
    async fn rollback_discards_uncommitted_writes() -> TbResult<()> {
        let mut store = MemStore::new();
        let before = store.snapshot();

        store.create("Ada", "Lovelace", &None).await?;
        store.rollback().await?;

        assert_eq!(store.snapshot(), before);
        Ok(())
    }

    /// The final write of an attach fails; the earlier writes (part
    /// timestamps, the attachment row) are partial state. A rollback
    /// discards all of it, leaving the committed state untouched (issue
    /// #409).
    #[tokio::test]
    async fn failed_attach_rolls_back_to_committed_state() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let before = store.snapshot();
        let s = test_session();

        // The last write of the attach flow fails, like a database failure.
        store.fail_next(Fault::UsageUpdate);

        let bike = fixture_bike(&s, &mut store).await?;
        let wheel = Part::create(
            "Spare Wheel".into(),
            "Zipp".into(),
            "404".into(),
            FRONT_WHEEL,
            None,
            T - Duration::days(1),
            &s,
            &mut store,
        )
        .await?;

        let att = Attachment::new(wheel.id, T, bike.id, FRONT_WHEEL, crate::MAX_TIME);
        let err = att
            .create(&mut store)
            .await
            .expect_err("the armed fault must fail the attach");
        assert!(matches!(err, Error::DatabaseFailure(_)));

        // The partial writes of the transaction are visible in the store…
        assert_ne!(
            store.snapshot(),
            before,
            "partial state must exist before the rollback"
        );

        // …and the rollback restores the committed state exactly.
        store.rollback().await?;
        assert_eq!(store.snapshot(), before);
        Ok(())
    }

    /// Same shape, earlier failure: the attach row never lands, but the
    /// part-timestamp write still happened. The rollback discards it (issue
    /// #409).
    #[tokio::test]
    async fn failed_attach_before_row_rolls_back() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let before = store.snapshot();
        let s = test_session();

        store.fail_next(Fault::AttachmentCreate);

        let bike = fixture_bike(&s, &mut store).await?;
        let wheel = Part::create(
            "Spare Wheel".into(),
            "Zipp".into(),
            "404".into(),
            FRONT_WHEEL,
            None,
            T - Duration::days(1),
            &s,
            &mut store,
        )
        .await?;

        let att = Attachment::new(wheel.id, T, bike.id, FRONT_WHEEL, crate::MAX_TIME);
        let err = att
            .create(&mut store)
            .await
            .expect_err("the armed fault must fail the attach");
        assert!(matches!(err, Error::DatabaseFailure(_)));

        assert_ne!(
            store.snapshot(),
            before,
            "the part-timestamp write must be partial state"
        );

        store.rollback().await?;
        assert_eq!(store.snapshot(), before);
        Ok(())
    }
}
