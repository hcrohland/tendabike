//! Rebuilds `backend/domain/src/prepopulated_data.rs` from the canonical
//! workshop store.
//!
//! The fixture builder is re-based on a scratch Postgres DB (issue #647):
//! the canonical workshop's domain-operation sequence runs against a
//! `PostgresStore` (a `SqlxConn` over the scratch database), the rows are
//! read back into `StoreSnapshot`, and serialized. Codegen requires a
//! scratch Postgres DB (set `SCRATCH_DATABASE_URL`), consistent with the
//! `#[ignore]`d sqlx suites.
//!
//! Run: `cargo run -p tb_sqlx --bin build-snapshot`

use sqlx::migrate::MigrateDatabase;
use std::time::Duration;

use tb_domain::{
    ActTypeId, Activity, ActivityId, ActivityStore, AttachmentStore, Part, PartId, PartStore,
    PartTypeId, Session, StoreSnapshot, UsageId, UsageStore, UserId, UserStore, attach_assembly,
};
use tb_sqlx::DbPool;
use time::OffsetDateTime;

/// The scratch database url from the environment (the codegen requires a
/// scratch Postgres DB, consistent with the `#[ignore]`d sqlx suites).
fn scratch_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("SCRATCH_DATABASE_URL").expect(
        "SCRATCH_DATABASE_URL is not set — the codegen requires a scratch \
         Postgres DB; set it to a disposable database URL and re-run",
    )
}

/// Bound one scratch-database step so a blackholed endpoint fails in
/// seconds instead of hanging the run.
async fn bounded<T>(
    fut: impl std::future::Future<Output = Result<T, sqlx::Error>>,
) -> Result<T, String> {
    match tokio::time::timeout(Duration::from_secs(10), fut).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("timed out after 10s".to_string()),
    }
}

/// The fixed attach time for the fixture (the same value the in-memory
/// builder used).
const ATTACH_TIME: OffsetDateTime = time::macros::datetime!(2023-01-01 00:00 UTC);

/// A sample purchase date (the same value the in-memory builder used).
fn sample_purchase_date() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// A test session with a fixed identity (the fixture's one user).
struct TestSession;

impl Session for TestSession {
    fn user_id(&self) -> UserId {
        UserId::from(1)
    }
    fn shop(&self) -> Option<tb_domain::ShopId> {
        None
    }
    fn is_admin(&self) -> bool {
        false
    }
}

/// Create a part like the in-memory builder did.
async fn mk_part(
    session: &dyn Session,
    name: &str,
    vendor: &str,
    model: &str,
    typ: PartTypeId,
    store: &mut tb_sqlx::SqlxConn<'_>,
) -> tb_domain::TbResult<Part> {
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
    id: ActivityId,
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
        user_id: UserId::from(1),
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

/// Run the canonical workshop's domain-operation sequence against a
/// `PostgresStore` (a `SqlxConn` over the scratch database) and read the
/// rows back into a `StoreSnapshot`.
async fn build_workshop_snapshot(pool: &DbPool) -> tb_domain::TbResult<StoreSnapshot> {
    let mut store = pool.begin().await?;
    let s = TestSession;

    // Create user 1 (the customer who owns all workshop parts).
    store.user_create("Tenda", "Bike", &None).await?;

    // ─── Bike A: Main Bike ────────────────────────────────────────────
    let bike_a = Part::create(
        "Main Bike".into(),
        "TendaBike".into(),
        "Standard Frame".into(),
        part_type::BIKE,
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
        part_type::FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let rw_a = mk_part(
        &s,
        "Rear Wheel A",
        "DT Swiss",
        "XR 1501",
        part_type::REAR_WHEEL,
        &mut store,
    )
    .await?;
    let ch_a = mk_part(
        &s,
        "Chain A",
        "Shimano",
        "CN-M510",
        part_type::CHAIN,
        &mut store,
    )
    .await?;
    let t_a1 = mk_part(
        &s,
        "Tire Front A",
        "Continental",
        "GP5000",
        part_type::TIRE,
        &mut store,
    )
    .await?;
    let t_a2 = mk_part(
        &s,
        "Tire Rear A",
        "Continental",
        "GP5000 ST",
        part_type::TIRE,
        &mut store,
    )
    .await?;

    attach_assembly(
        &s,
        fw_a.id,
        ATTACH_TIME,
        bike_a.id,
        part_type::BIKE,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        rw_a.id,
        ATTACH_TIME,
        bike_a.id,
        part_type::BIKE,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        ch_a.id,
        ATTACH_TIME,
        bike_a.id,
        part_type::BIKE,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        t_a1.id,
        ATTACH_TIME,
        bike_a.id,
        part_type::FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        t_a2.id,
        ATTACH_TIME,
        bike_a.id,
        part_type::REAR_WHEEL,
        false,
        &mut store,
    )
    .await?;

    // ─── Bike B: Road Bike ────────────────────────────────────────────
    let bike_b = Part::create(
        "Road Bike".into(),
        "Trek".into(),
        "Domane".into(),
        part_type::BIKE,
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
        part_type::FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let rw_b = mk_part(
        &s,
        "Rear Wheel B",
        "Mavic",
        "Carbon WS",
        part_type::REAR_WHEEL,
        &mut store,
    )
    .await?;
    let ch_b = mk_part(
        &s,
        "Chain B",
        "SRAM",
        "PC-XX1",
        part_type::CHAIN,
        &mut store,
    )
    .await?;
    let t_b1 = mk_part(
        &s,
        "Tire Front B",
        "Schwalbe",
        "One",
        part_type::TIRE,
        &mut store,
    )
    .await?;
    let t_b2 = mk_part(
        &s,
        "Tire Rear B",
        "Schwalbe",
        "One Plus",
        part_type::TIRE,
        &mut store,
    )
    .await?;

    attach_assembly(
        &s,
        fw_b.id,
        ATTACH_TIME,
        bike_b.id,
        part_type::BIKE,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        rw_b.id,
        ATTACH_TIME,
        bike_b.id,
        part_type::BIKE,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        ch_b.id,
        ATTACH_TIME,
        bike_b.id,
        part_type::BIKE,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        t_b1.id,
        ATTACH_TIME,
        bike_b.id,
        part_type::FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        t_b2.id,
        ATTACH_TIME,
        bike_b.id,
        part_type::REAR_WHEEL,
        false,
        &mut store,
    )
    .await?;

    // ─── Spares (loose; the spare wheel is not mounted, just carries its tire) ───
    let _sc1 = mk_part(
        &s,
        "Spare Chain 1",
        "Shimano",
        "HG-54",
        part_type::CHAIN,
        &mut store,
    )
    .await?;
    let _sc2 = mk_part(
        &s,
        "Spare Chain 2",
        "SRAM",
        "PC-1031",
        part_type::CHAIN,
        &mut store,
    )
    .await?;
    let _st = mk_part(
        &s,
        "Spare Tire",
        "Continental",
        "Supersonic",
        part_type::TIRE,
        &mut store,
    )
    .await?;

    let sw = mk_part(
        &s,
        "Spare Wheel",
        "HED",
        "Stinger 3",
        part_type::FRONT_WHEEL,
        &mut store,
    )
    .await?;
    let st2 = mk_part(
        &s,
        "Spare Wheel Tire",
        "Continental",
        "GP5000",
        part_type::TIRE,
        &mut store,
    )
    .await?;
    attach_assembly(
        &s,
        st2.id,
        ATTACH_TIME,
        sw.id,
        part_type::FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;

    // ─── Activities on Bike A (for usage calculation) ─────────────────
    let base = sample_purchase_date() - time::Duration::days(180);
    mk_activity(
        ActivityId::new(1),
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
        ActivityId::new(2),
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
        ActivityId::new(3),
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

    // Compute usage from activities — populates usages for Bike A + all attached subparts.
    Activity::rescan_all(&mut store).await?;

    // Read the rows back into a `StoreSnapshot`.
    let user = UserStore::user_get(&mut store, UserId::from(1)).await?;
    let users = vec![user];

    let parts = PartStore::part_get_all_for_userid(&mut store, &UserId::from(1)).await?;

    let mut attachments = vec![];
    for part in &parts {
        let part_atts = AttachmentStore::attachments_all_by_part(&mut store, part.id).await?;
        attachments.extend(part_atts);
    }

    // Collect usage ids from parts and attachments, then read each.
    let mut usage_ids: Vec<UsageId> = parts.iter().map(|p| p.usage).collect();
    for att in &attachments {
        usage_ids.push(att.usage);
    }
    usage_ids.sort();
    usage_ids.dedup();
    let mut usages = vec![];
    for uid in usage_ids {
        if let Some(usage) = UsageStore::usage_get(&mut store, uid).await? {
            usages.push(usage);
        }
    }

    let activities = ActivityStore::activity_get_really_all(&mut store).await?;

    // Commit the transaction.
    store.commit().await?;

    // Sort deterministically (the same ordering the in-memory builder used).
    let mut snap = StoreSnapshot {
        users,
        parts,
        attachments,
        usages,
        activities,
    };
    snap.users.sort_by_key(|u| i32::from(u.id));
    snap.parts.sort_by_key(|p| p.id);
    snap.attachments.sort_by_key(|a| (a.part_id, a.attached));
    snap.usages.sort_by_key(|u| u.id);
    snap.activities.sort_by_key(|a| a.id);
    Ok(snap)
}

/// Part type id constants (the same values the in-memory builder used).
mod part_type {
    use tb_domain::PartTypeId;

    pub const BIKE: PartTypeId = PartTypeId::from_id(1);
    pub const FRONT_WHEEL: PartTypeId = PartTypeId::from_id(2);
    pub const TIRE: PartTypeId = PartTypeId::from_id(3);
    pub const CHAIN: PartTypeId = PartTypeId::from_id(4);
    pub const REAR_WHEEL: PartTypeId = PartTypeId::from_id(5);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let url = scratch_url();

    // Create the scratch database (force-drop any leftover from a previous
    // run), then run migrations through the same `DbPool::new` the app uses.
    let exists = bounded(sqlx::Postgres::database_exists(&url))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if exists {
        bounded(sqlx::Postgres::force_drop_database(&url))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    bounded(sqlx::Postgres::create_database(&url))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let pool = DbPool::new(&url)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let snap = build_workshop_snapshot(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!(
        "Parts: {}, attachments: {}, usages: {}, activities: {}",
        snap.parts.len(),
        snap.attachments.len(),
        snap.usages.len(),
        snap.activities.len()
    );

    let json = serde_json::to_string_pretty(&snap)?;

    let content = format!(
        "//! Prepopulated store data for tests.\n\
         //! Generated by the `build-snapshot` bin.\n\
         //! Regenerate: cargo run -p tb_sqlx --bin build-snapshot\n\
         \n\
         /// Formatted JSON snapshot of the prepopulated store.\n\
         /// This is a pretty-printed JSON string that gets deserialized into `StoreSnapshot`.\n\
         pub const SNAPSHOT_JSON: &str = r#\"{}\"#;\n",
        json
    );

    // Write to the `tb_domain` crate's `prepopulated_data.rs` (the bin lives
    // in `tb_sqlx`, but the generated file is in `tb_domain`).
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../domain/src/prepopulated_data.rs"
    );
    std::fs::write(path, &content)?;
    println!("Wrote {}", path);

    Ok(())
}
