// This file is part of TendaBike.
//
// TendaBike is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// TendaBike is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with TendaBike. If not, see <https://www.gnu.org/licenses/>.

//! Store seam integration suite (issue #406).
//!
//! Loads the standard prepopulated domain fixture (the same snapshot the
//! in-memory suite in `tb_domain` runs against) into a real Postgres database
//! and drives a representative set of domain operations through the
//! `SqlxConn` store — attachment attach, merge, and delete; activity upsert,
//! update, delete, and lookup; and the user summary read — asserting the same
//! domain-level results the in-memory suite asserts.
//!
//! The attachment path rules — successor, adjacent-merge trigger, delete
//! identity — were unified on the Postgres rule (the source of truth) in
//! #407, and the activity path rules — update field preservation,
//! time-range boundary, and the user+time minute match — in #408, so every
//! test in this suite asserts the one rule per operation that both stores
//! apply.
//!
//! The suite is deterministic: all tests run serialized against one database;
//! a one-time seed loads the fixture, and every test opens a transaction
//! with the sequences reset just past the fixture ids and rolls back, so
//! each test starts from exactly the prepopulated state.
//!
//! The suite requires a disposable database reachable at `DATABASE_URL`
//! (falling back to `.env`). When no `DATABASE_URL` is configured, every
//! test skips itself, so the plain `cargo test --workspace` job (no
//! database) and the existing in-memory suites stay green.
//!
//! The CI job that runs this suite against a real Postgres service is the
//! informational (non-blocking) `postgres-seam` job in
//! `.github/workflows/test.yml`.

use std::collections::HashSet;

use tb_domain::test_support::{MemStore, StoreSnapshot, TestSession, part_type_ids};
use tb_domain::{
    ActTypeId, Activity, ActivityId, ActivityStore, Attachment, AttachmentStore, MAX_TIME, Part,
    PartId, PartStore, PartTypeId, Store, Usage, UsageId, UsageStore, UserId, UserStore,
    attach_assembly, detach_assembly, dispose_assembly, round_time,
};
use time::{OffsetDateTime, macros::datetime};
use tokio::sync::{Mutex, MutexGuard, OnceCell};
use uuid::Uuid;

use part_type_ids::{BIKE, CHAIN, FRONT_WHEEL, TIRE};

// ---------------------------------------------------------------------------
// Plumbing: shared pool, serialization, one-time fixture seed
// ---------------------------------------------------------------------------

static LOCK: Mutex<()> = Mutex::const_new(());
static SEEDED: OnceCell<()> = OnceCell::const_new();

/// The database url from the environment or `.env`, if any.
fn database_url() -> Option<String> {
    let _ = dotenvy::dotenv();
    std::env::var("DATABASE_URL").ok()
}

/// A fresh pool for this test's runtime, or `None` when no `DATABASE_URL` is
/// configured (the test then skips itself).
///
/// Each `#[tokio::test]` runs on its own runtime, so a pool must never outlive
/// the runtime that created it: the pool's background tasks (connection
/// returns, maintenance) are spawned on that runtime and die with it, which
/// leaves the pool's slot accounting in a state where the next runtime's
/// `acquire` waits the full acquire timeout. Every test therefore opens its
/// own pool and drops it with the test.
async fn pool() -> Option<tb_sqlx::DbPool> {
    let url = database_url()?;
    tb_sqlx::DbPool::new(&url).await.ok()
}

fn db_err(err: sqlx::Error) -> tb_domain::Error {
    tb_domain::Error::DatabaseFailure(err.into())
}

/// A test handle: one transaction on the shared test database. The lock is
/// held for the whole test so all tests in this suite run serialized; the
/// transaction is rolled back explicitly at the end of every test.
struct Seam {
    /// Declared first so it is dropped last: the lock stays held while the
    /// test's transaction is rolled back at the end of the test.
    _lock: MutexGuard<'static, ()>,
    store: tb_sqlx::SqlxConn<'static>,
}

/// Open a fresh fixture transaction for a test, or `None` when this machine
/// has no `DATABASE_URL` (the test then skips itself).
///
/// The pool is local to this test: it is dropped when the `Seam` is built,
/// together with the test's runtime — the transaction keeps the pool alive
/// internally through its own `Arc` handle until it is rolled back.
async fn seam() -> Option<Seam> {
    // The lock first: the pool connection (and its no-op migration run) is
    // taken while the lock is held, so tests never contend for the database.
    let lock = LOCK.lock().await;
    let pool = pool().await?;
    SEEDED
        .get_or_try_init(|| async { seed(&pool).await })
        .await
        .ok()?;
    let mut store = pool.begin().await.ok()?;
    reset_sequences(&mut store).await.ok()?;
    Some(Seam { store, _lock: lock })
    // `pool` is dropped here, with this test's runtime.
}

/// Row for the pre-truncate state report.
#[derive(sqlx::FromRow)]
struct PreTruncate {
    users: i64,
    parts: i64,
}

/// Truncate every table and load the standard prepopulated fixture (the same
/// snapshot the in-memory suite uses), then point the sequences just past
/// the fixture ids. The fixture is committed once; every test afterwards
/// works in its own transaction and rolls back.
async fn seed(pool: &tb_sqlx::DbPool) -> tb_domain::TbResult<()> {
    let mut tx = pool.begin().await?;

    // Report the pre-truncate state so a run against a database that holds
    // more than the standard fixture is visible in the logs.
    let pre = sqlx::query_as::<_, PreTruncate>(
        "SELECT (SELECT count(*) FROM users) AS users, (SELECT count(*) FROM parts) AS parts",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    eprintln!(
        "store-seam: seeding; pre-truncate state users={} parts={} \
        (expected 1 user, 17 parts)",
        pre.users, pre.parts
    );

    sqlx::query(
        "TRUNCATE users, parts, usages, attachments, activities, services,
                 service_plans, shops, shop_subscriptions, part_notes
         RESTART IDENTITY CASCADE",
    )
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    let snap = MemStore::prepopulated().snapshot();
    load_fixture(&mut tx, &snap).await?;

    // The fixture uses explicit ids 1..=17 (parts) and 1 (user); the
    // sequences must continue just past them.
    sqlx::query(
        "SELECT setval(pg_get_serial_sequence('parts', 'id'), 17),
                setval(pg_get_serial_sequence('users', 'id'), 1)",
    )
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    tx.commit().await
}

/// Load the snapshot into the database: users and parts through raw inserts
/// (the trait cannot set their serial ids; the column mapping mirrors
/// `DbUser`/`DbPart` in `store/user.rs`/`store/part.rs`), everything else
/// through the existing domain-to-row conversions in the store traits.
async fn load_fixture(
    tx: &mut tb_sqlx::SqlxConn<'_>,
    snap: &StoreSnapshot,
) -> tb_domain::TbResult<()> {
    let exec: &mut sqlx::PgConnection = tx;

    for user in &snap.users {
        sqlx::query(
            "INSERT INTO users (id, name, firstname, is_admin, avatar, onboarding_status)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(i32::from(user.id))
        .bind(&user.name)
        .bind(&user.firstname)
        .bind(user.is_admin)
        .bind(&user.avatar)
        .bind(i32::from(user.onboarding_status))
        .execute(&mut *exec)
        .await
        .map_err(db_err)?;
    }

    for part in &snap.parts {
        sqlx::query(
            "INSERT INTO parts (id, owner, what, name, vendor, model, purchase, last_used,
                                disposed_at, usage, source, shop)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(i32::from(part.id))
        .bind(i32::from(part.owner))
        .bind(i32::from(part.what))
        .bind(&part.name)
        .bind(&part.vendor)
        .bind(&part.model)
        .bind(part.purchase)
        .bind(part.last_used)
        .bind(part.disposed_at)
        .bind(Uuid::from(part.usage))
        .bind(&part.source)
        .bind(part.shop.map(i32::from))
        .execute(&mut *exec)
        .await
        .map_err(db_err)?;
    }

    for att in &snap.attachments {
        tx.attachment_create(*att).await?;
    }
    UsageStore::update(tx, &snap.usages).await?;
    for act in &snap.activities {
        tx.activity_create(act.clone()).await?;
    }
    Ok(())
}

/// Point the sequences just past the fixture ids inside the test
/// transaction, so freshly created parts/users get the same ids the
/// in-memory store hands out (18 for parts, 2 for users). Sequence changes
/// are non-transactional, so each test resets them on entry.
async fn reset_sequences(tx: &mut tb_sqlx::SqlxConn<'_>) -> tb_domain::TbResult<()> {
    sqlx::query(
        "SELECT setval(pg_get_serial_sequence('parts', 'id'), 17),
                setval(pg_get_serial_sequence('users', 'id'), 1)",
    )
    .execute(&mut ***tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Sample data helpers, mirroring the in-memory suite's helpers
// ---------------------------------------------------------------------------

fn test_session() -> TestSession {
    TestSession::new(UserId::from(1))
}

/// The in-memory suite's part purchase date (2023-11-14T22:13:20Z).
fn sample_purchase_date() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// The in-memory suite's first attachment time (2024-01-01).
fn attachment_time() -> OffsetDateTime {
    datetime!(2024-01-01 00:00 UTC)
}

/// The in-memory suite's later attachment time (2024-06-01).
fn later_time() -> OffsetDateTime {
    datetime!(2024-06-01 00:00 UTC)
}

/// The in-memory suite's activity start (2023-11-14T22:13:20Z).
fn activity_start() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// An attachment with a fresh usage id, as `Attachment::new` builds it (that
/// constructor is `pub(crate)` in the domain, so the seam builds the same
/// value from the public fields).
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
        usage: UsageId::from(Uuid::now_v7()),
    }
}

/// Create a part like the in-memory tests do.
async fn create_part(
    name: &str,
    vendor: &str,
    model: &str,
    what: PartTypeId,
    store: &mut tb_sqlx::SqlxConn<'_>,
) -> Part {
    Part::create(
        name.to_string(),
        vendor.to_string(),
        model.to_string(),
        what,
        None,
        sample_purchase_date(),
        &test_session(),
        store,
    )
    .await
    .expect("creating a part works")
}

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// The standard prepopulated fixture, the same snapshot the in-memory suite
/// uses.
fn fixture() -> StoreSnapshot {
    MemStore::prepopulated().snapshot()
}

/// The prepopulated fixture loads back from the database unchanged.
#[tokio::test]
async fn fixture_roundtrip() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let snap = fixture();

    // users
    let user = UserStore::get(&mut store, UserId::from(1)).await?;
    assert_eq!(user.name, snap.users[0].name);
    assert_eq!(user.firstname, snap.users[0].firstname);
    assert_eq!(user.is_admin, snap.users[0].is_admin);
    assert_eq!(user.avatar, snap.users[0].avatar);
    assert_eq!(user.onboarding_status, snap.users[0].onboarding_status);

    // parts
    for part in &snap.parts {
        let stored = store.partid_get_part(part.id).await?;
        assert_eq!(&stored, part, "part {} roundtrip differs", part.id);
    }

    // attachments
    let mut counted = 0;
    for part in &snap.parts {
        let stored = store.attachments_all_by_part(part.id).await?;
        let expected = snap
            .attachments
            .iter()
            .filter(|a| a.part_id == part.id)
            .collect::<Vec<_>>();
        assert_eq!(
            stored.len(),
            expected.len(),
            "part {} attachment count",
            part.id
        );
        for expected in &expected {
            assert!(
                stored.iter().any(|a| a == *expected),
                "part {} is missing one of its attachments",
                part.id
            );
        }
        counted += stored.len();
    }
    assert_eq!(counted, snap.attachments.len());

    // usages
    for usage in &snap.usages {
        let stored = UsageStore::get(&mut store, usage.id).await?;
        assert_eq!(stored.as_ref(), Some(usage), "usage {} differs", usage.id);
    }

    // activities
    let acts = store.get_all(&UserId::from(1)).await?;
    assert_eq!(acts.len(), snap.activities.len());
    for expected in &snap.activities {
        assert!(
            acts.iter().any(|a| a == expected),
            "activity {} ({}) missing",
            expected.id,
            expected.name
        );
    }

    store.rollback().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// User summary read
// ---------------------------------------------------------------------------

/// The user summary read returns the full fixture content.
#[tokio::test]
async fn user_summary_read() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let summary = UserId::from(1).get_summary(None, &mut store).await?;

    // The fixture content: counts only (the stores order vectors differently).
    assert_eq!(summary.parts.len(), 17);
    assert_eq!(summary.activities.len(), 3);
    assert_eq!(summary.attachments.len(), 11);
    // 17 part usages + 11 attachment usages (missing usage rows read as
    // zeros, identically on both stores).
    assert_eq!(summary.usages.len(), 28);
    assert!(summary.shops.is_empty());
    assert!(summary.users.is_empty());
    assert!(summary.services.is_empty());
    assert!(summary.plans.is_empty());
    assert!(summary.part_notes.is_empty());

    // Field lookups: the "Chain A" part, its attachment, and its usage.
    let chain = summary
        .parts
        .iter()
        .find(|p| p.id == PartId::from(4))
        .unwrap();
    assert_eq!(chain.name, "Chain A");
    let att = summary
        .attachments
        .iter()
        .find(|a| a.a.part_id == PartId::from(4))
        .unwrap();
    assert_eq!(att.a.hook, BIKE);
    assert_eq!(att.a.detached, MAX_TIME);
    let usage = summary.usages.iter().find(|u| u.id == chain.usage).unwrap();
    assert_eq!(usage.time, 8025);
    assert_eq!(usage.count, 3);

    store.rollback().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Attachment: attach
// ---------------------------------------------------------------------------

/// Attaching a part to a hook with nothing on it creates a single
/// still-attached row and bumps the part's last_used (mirrors
/// `attach_assembly_attaches_part_to_gear`).
#[tokio::test]
async fn attach_new_part_to_empty_hook() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();

    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    let summary = attach_assembly(
        &session,
        chain.id,
        attachment_time(),
        bike.id,
        BIKE,
        false,
        &mut store,
    )
    .await?;
    assert!(!summary.parts.is_empty());

    let att = store
        .attachment_get_by_part_and_time(chain.id, attachment_time())
        .await?
        .expect("the chain is attached");
    assert_eq!(att.gear, bike.id);
    assert_eq!(att.detached, MAX_TIME);

    // The part's last_used is bumped to the attach time.
    let chain = store.partid_get_part(chain.id).await?;
    assert_eq!(chain.last_used, attachment_time());

    store.rollback().await?;
    Ok(())
}

/// The flat row model (ADR-0003): a tire mounted onto a front wheel that is
/// itself on the bike is stored against the top-level gear (mirrors
/// `attach_assembly_resolves_mounted_gear_to_top_level`).
#[tokio::test]
async fn attach_resolves_mounted_gear_to_top_level() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();
    let time = attachment_time();

    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let wheel = create_part(
        "Front Wheel",
        "Fulcrum",
        "Rapid 150",
        FRONT_WHEEL,
        &mut store,
    )
    .await;
    let tire = create_part("Front Tire", "Schwalbe", "One", TIRE, &mut store).await;

    // mount the wheel on the bike first
    let _ = attach_assembly(&session, wheel.id, time, bike.id, BIKE, false, &mut store).await?;
    // attach the tire to the *wheel*; the domain resolves the top-level gear
    let _ = attach_assembly(
        &session,
        tire.id,
        time,
        wheel.id,
        FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;

    let att = store
        .attachment_get_by_part_and_time(tire.id, time)
        .await?
        .expect("the tire is attached");
    assert_eq!(att.gear, bike.id);
    assert_eq!(att.hook, FRONT_WHEEL);

    store.rollback().await?;
    Ok(())
}

/// Re-attaching a part at the time its row started deletes and re-creates
/// the row: exactly one still-attached row remains (mirrors
/// `attach_assembly_auto_detaches_and_reattaches_same_part`).
#[tokio::test]
async fn attach_reattach_at_own_time() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();

    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    for _ in 0..3 {
        let summary = attach_assembly(
            &session,
            chain.id,
            attachment_time(),
            bike.id,
            BIKE,
            false,
            &mut store,
        )
        .await?;
        assert!(!summary.parts.is_empty());
    }

    let atts = store.attachments_all_by_part(chain.id).await?;
    assert_eq!(atts.len(), 1, "re-attaching replaces the row");
    assert_eq!(atts[0].gear, bike.id);
    assert_eq!(atts[0].detached, MAX_TIME);

    store.rollback().await?;
    Ok(())
}

/// Attaching a part detaches the different part already occupying the hook
/// (mirrors `attach_assembly_detaches_predecessor_on_gear`).
#[tokio::test]
async fn attach_detaches_predecessor() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();

    let bike1 = create_part("Bike 1", "TendaBike", "Standard", BIKE, &mut store).await;
    let _bike2 = create_part("Bike 2", "TendaBike", "Pro", BIKE, &mut store).await;
    let chain1 = create_part("Chain 1", "Shimano", "CN-M510", CHAIN, &mut store).await;
    let chain2 = create_part("Chain 2", "KMC", "X10", CHAIN, &mut store).await;

    // Attach chain1 to bike1 at attachment_time (raw row, like the in-memory test).
    store
        .attachment_create(att(chain1.id, attachment_time(), bike1.id, BIKE, MAX_TIME))
        .await?;

    // Attach chain2 to bike1 at later_time (should detach chain1 from bike1)
    let _ = attach_assembly(
        &session,
        chain2.id,
        later_time(),
        bike1.id,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    // chain1 is cut off on bike1 at later_time
    let chain1_atts = store.attachments_all_by_part(chain1.id).await?;
    let chain1_att = chain1_atts
        .iter()
        .find(|a| a.gear == bike1.id)
        .expect("chain1 was attached to bike1");
    assert_eq!(chain1_att.detached, later_time());

    // chain2 is attached to bike1 at later_time
    let chain2_att = store
        .attachment_get_by_part_and_time(chain2.id, later_time())
        .await?
        .expect("chain2 is attached at later_time");
    assert_eq!(chain2_att.gear, bike1.id);

    store.rollback().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Attachment: merge and delete (rules unified in #407)
// ---------------------------------------------------------------------------

/// Re-attaching a part at the time its row ended continues the same row —
/// the adjacent-merge (unified rule #407, one rule on both stores).
#[tokio::test]
async fn attach_merge_adjacent_with_previous() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();
    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    // First attachment t1..t2, then re-attach exactly at t2 (works on both
    // stores; the resulting rows differ).
    store
        .attachment_create(att(
            chain.id,
            attachment_time(),
            bike.id,
            BIKE,
            later_time(),
        ))
        .await?;
    let _ = attach_assembly(
        &session,
        chain.id,
        later_time(),
        bike.id,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    // The adjacent rows become one: the previous row was continued, not
    // duplicated.
    let atts = store.attachments_all_by_part(chain.id).await?;
    assert_eq!(atts.len(), 1, "adjacent rows merge into one");
    assert_eq!(atts[0].attached, attachment_time());
    assert_eq!(atts[0].detached, MAX_TIME);

    store.rollback().await?;
    Ok(())
}

/// An attachment delete addresses its row by part + attach time — the
/// database's key — so a stale gear field in the argument does not change
/// which row is deleted (unified rule #407).
#[tokio::test]
async fn attachment_delete_identity() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike1 = create_part("Bike 1", "TendaBike", "Standard", BIKE, &mut store).await;
    let bike2 = create_part("Bike 2", "TendaBike", "Pro", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    let t1 = attachment_time();
    let a1 = att(chain.id, t1, bike1.id, BIKE, MAX_TIME);
    AttachmentStore::attachment_create(&mut store, a1).await?;

    // A stale gear field in the argument — the row is still addressed by
    // part + attach time and is deleted.
    let stale = Attachment {
        gear: bike2.id,
        ..a1
    };
    let deleted = AttachmentStore::delete(&mut store, stale).await?;
    assert_eq!(deleted.gear, bike1.id);

    let atts = store.attachments_all_by_part(chain.id).await?;
    assert!(atts.is_empty(), "the row was deleted");

    store.rollback().await?;
    Ok(())
}

/// Finding the successor of an attachment: another part, of the same type,
/// at the same hook, attached later — the part's own later rows never count
/// (unified rule #407).
#[tokio::test]
async fn attachment_find_successor() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;
    let chain2 = create_part("Chain 2", "KMC", "X10", CHAIN, &mut store).await;

    // chain: t1..t2 and its own later row t2..MAX (never its successor);
    // chain2 takes the same hook at t2.
    store
        .attachment_create(att(
            chain.id,
            attachment_time(),
            bike.id,
            CHAIN,
            later_time(),
        ))
        .await?;
    store
        .attachment_create(att(chain.id, later_time(), bike.id, CHAIN, MAX_TIME))
        .await?;
    store
        .attachment_create(att(chain2.id, later_time(), bike.id, CHAIN, MAX_TIME))
        .await?;

    let successor = store
        .attachment_find_successor(chain.id, bike.id, CHAIN, attachment_time(), CHAIN)
        .await?
        .expect("the other chain's later row is the successor");
    assert_eq!(successor.part_id, chain2.id);
    assert_eq!(successor.attached, later_time());

    store.rollback().await?;
    Ok(())
}

/// Finding the attachment already attached to a part: the row of this part
/// at this gear and hook that ended exactly at the query time — the
/// adjacent-merge trigger (unified rule #407).
#[tokio::test]
async fn attachment_find_part_attached_already() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    // Row t1..t2.
    store
        .attachment_create(att(
            chain.id,
            attachment_time(),
            bike.id,
            CHAIN,
            later_time(),
        ))
        .await?;

    // The row covers t1 but does not end there — not the merge trigger.
    let found = store
        .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, attachment_time())
        .await?;
    assert!(found.is_none());

    // The row ends exactly at t2 — the adjacent-merge trigger.
    let found = store
        .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, later_time())
        .await?
        .expect("the row ending exactly at the time is the merge trigger");
    assert_eq!(found.attached, attachment_time());

    store.rollback().await?;
    Ok(())
}

/// `find_part_attached_already` for a part that was never attached is
/// `None` on both stores (no divergence).
#[tokio::test]
async fn find_attached_already_never_attached() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    let found = store
        .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, attachment_time())
        .await?;
    assert!(found.is_none());

    store.rollback().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Attachment: detach and dispose
// ---------------------------------------------------------------------------

/// Detaching a fixture part re-cuts the attachment at the detach time and
/// re-derives its usage from the activities inside the new window (mirrors
/// `detach_assembly_recalculates_usage_excluding_activity_after_detach`).
#[tokio::test]
async fn detach_recuts_and_recalculates_usage() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();
    let wheel = PartId::from(2);

    // The latest prepopulated activity and its 15-minute floor.
    let latest = datetime!(2023-05-19 22:13:20 UTC);
    let detach_at = round_time(latest);
    assert_eq!(detach_at, datetime!(2023-05-19 22:00:00 UTC));

    // The attachment row and its usage before the detach (the row is
    // replaced by a new one and the old usage row is deleted).
    let old_usage = store
        .attachment_get_by_part_and_time(wheel, detach_at - time::Duration::hours(1))
        .await?
        .expect("the wheel is attached before the detach")
        .usage;

    let _ = detach_assembly(&session, wheel, detach_at, false, &mut store).await?;

    // The attachment is cut at the detach time and gone from then on.
    let att = store
        .attachment_get_by_part_and_time(wheel, detach_at - time::Duration::hours(1))
        .await?
        .expect("the wheel is attached before the detach");
    assert_eq!(att.attached, datetime!(2023-01-01 00:00 UTC));
    assert_eq!(att.detached, detach_at);
    assert!(
        store
            .attachment_get_by_part_and_time(wheel, latest)
            .await?
            .is_none(),
        "no attachment of the wheel from the detach time on"
    );

    // The wheel usage is recalculated from the two earlier rides only
    // (25+5200, 50000+40000, 400+600, 400+600, 500+500, 1+1);
    // descend is None in the activities, so it falls back to climb.
    let wheel_usage = store.partid_get_part(wheel).await?.usage;
    let stored = UsageStore::get(&mut store, wheel_usage)
        .await?
        .expect("the wheel usage row exists");
    assert_eq!(
        stored,
        Usage {
            id: wheel_usage,
            time: 5225,
            distance: 90000,
            climb: 1000,
            descend: 1000,
            energy: 1000,
            count: 2,
        }
    );

    // The replacement attachment usage row is a separate row with its own
    // id but carries the same recalculated values.
    let att_usage = UsageStore::get(&mut store, att.usage)
        .await?
        .expect("the replacement attachment usage exists");
    let mut expected = stored;
    expected.id = att_usage.id;
    assert_eq!(att_usage, expected);

    // The old attachment usage row is deleted.
    assert!(
        UsageStore::get(&mut store, old_usage).await?.is_none(),
        "the old attachment usage row is deleted"
    );

    // The other parts attached to the bike are untouched.
    for pid in [PartId::from(1), PartId::from(3), PartId::from(4)] {
        let uid = store.partid_get_part(pid).await?.usage;
        let stored = UsageStore::get(&mut store, uid)
            .await?
            .expect("the part usage row exists");
        assert_eq!(
            stored,
            Usage {
                id: uid,
                time: 8025,
                distance: 125000,
                climb: 1100,
                descend: 1100,
                energy: 1500,
                count: 3,
            }
        );
    }

    // The wheel's last_used is unchanged (the attach time is earlier).
    let wheel_part = store.partid_get_part(wheel).await?;
    assert_eq!(wheel_part.last_used, datetime!(2023-11-14 22:00 UTC));

    store.rollback().await?;
    Ok(())
}

/// Detaching a part that is not attached is a not-found error (mirrors
/// `detach_assembly_api_returns_error_if_not_attached`).
#[tokio::test]
async fn detach_not_attached_is_not_found() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    let result = detach_assembly(&session, chain.id, attachment_time(), false, &mut store).await;
    assert!(
        matches!(&result, Err(tb_domain::Error::NotFound(_))),
        "detaching a never-attached part must fail, got {result:?}"
    );

    store.rollback().await?;
    Ok(())
}

/// Disposing a loose part sets its disposed timestamp (mirrors
/// `dispose_assembly_disposes_part`).
#[tokio::test]
async fn dispose_part() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let session = test_session();
    let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

    // Dispose at later_time (no current attachment).
    let _ = dispose_assembly(&session, chain.id, later_time(), false, &mut store).await?;

    let part = store.partid_get_part(chain.id).await?;
    assert_eq!(part.disposed_at, Some(later_time()));

    store.rollback().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Activity: upsert, update, delete
// ---------------------------------------------------------------------------

/// Creating a new activity for the prepopulated bike accounts for the bike
/// and every part attached to it (mirrors
/// `activity_upsert_creates_new_accounts_bike_and_attached_parts`).
#[tokio::test]
async fn activity_upsert_creates_and_accounts() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike = PartId::from(1);
    let act = Activity {
        id: ActivityId::new(100),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: "New Ride".to_string(),
        start: activity_start(),
        duration: 3600,
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        gear: Some(bike),
        device_name: None,
        external_id: None,
    };

    let summary = act.clone().upsert(&test_session(), &mut store).await?;

    // The new activity is reported with the bike as gear.
    assert_eq!(summary.activities, vec![act]);

    // The bike and all attached parts: front wheel, rear wheel, chain, tires.
    let part_ids: HashSet<PartId> = summary.parts.iter().map(|p| p.id).collect();
    assert_eq!(
        part_ids,
        [1, 2, 3, 4, 5, 6].into_iter().map(PartId::from).collect()
    );
    // The bike's last_used is bumped to the activity start.
    let bike_part = summary.parts.iter().find(|p| p.id == bike).unwrap();
    assert_eq!(bike_part.last_used, round_time(activity_start()));

    // Every affected usage (6 part usages + 5 attachment usages) is
    // increased by the activity metrics (8025+1000, 125000+10000, 1100+100,
    // 1100+100, 1500+200, 3+1); descend is None in the activity, so it
    // falls back to climb.
    assert_eq!(summary.usages.len(), 11);
    let mut expected = Usage {
        id: UsageId::default(),
        time: 9025,
        distance: 135000,
        climb: 1200,
        descend: 1200,
        energy: 1700,
        count: 4,
    };
    for u in &summary.usages {
        expected.id = u.id;
        assert_eq!(*u, expected, "unexpected usage for {}", u.id);
    }

    // The updates are persisted in the store.
    let bike_usage = store.partid_get_part(bike).await?.usage;
    let stored = UsageStore::get(&mut store, bike_usage)
        .await?
        .expect("the bike usage row exists");
    assert_eq!(stored.time, 9025);
    assert_eq!(stored.distance, 135000);
    assert_eq!(stored.count, 4);

    store.rollback().await?;
    Ok(())
}

/// Deleting an activity reverts the create: usage back at baseline and the
/// activity reported zeroed (mirrors
/// `activity_delete_reverts_bike_and_attached_part_usage`).
#[tokio::test]
async fn activity_delete_reverts_usage() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike = PartId::from(1);
    let act = Activity {
        id: ActivityId::new(100),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: "New Ride".to_string(),
        start: activity_start(),
        duration: 3600,
        time: Some(1000),
        distance: Some(10000),
        climb: Some(100),
        descend: None,
        energy: Some(200),
        gear: Some(bike),
        device_name: None,
        external_id: None,
    };

    // Create the activity so its usage is accounted, then delete it.
    act.clone().upsert(&test_session(), &mut store).await?;
    let summary = ActivityId::new(100)
        .delete(&test_session(), &mut store)
        .await?;

    // The deleted activity is reported with its metrics zeroed.
    let mut expected = act;
    expected.gear = None;
    expected.duration = 0;
    expected.time = None;
    expected.distance = None;
    expected.climb = None;
    expected.descend = None;
    expected.energy = None;
    assert_eq!(summary.activities, vec![expected]);

    // The bike and all attached parts are affected again.
    let part_ids: HashSet<PartId> = summary.parts.iter().map(|p| p.id).collect();
    assert_eq!(
        part_ids,
        [1, 2, 3, 4, 5, 6].into_iter().map(PartId::from).collect()
    );

    // Every affected usage is back at the prepopulated baseline.
    let mut expected = Usage {
        id: UsageId::default(),
        time: 8025,
        distance: 125000,
        climb: 1100,
        descend: 1100,
        energy: 1500,
        count: 3,
    };
    for u in &summary.usages {
        expected.id = u.id;
        assert_eq!(*u, expected, "unexpected usage for {}", u.id);
    }

    // The updates are persisted in the store.
    let bike_usage = store.partid_get_part(bike).await?.usage;
    let stored = UsageStore::get(&mut store, bike_usage)
        .await?
        .expect("the bike usage row exists");
    assert_eq!(stored.time, 8025);
    assert_eq!(stored.count, 3);

    // The activity is gone.
    assert!(
        store
            .activity_read_by_id(ActivityId::new(100))
            .await?
            .is_none()
    );

    store.rollback().await?;
    Ok(())
}

/// Updating an activity returns a summary containing it (mirrors
/// `activity_update_returns_summary`). The in-memory suite updates an
/// activity whose id collides with the fixture's; the database primary key
/// forbids that, so the seam uses a fresh id — same domain-level assertion.
#[tokio::test]
async fn activity_update_returns_summary() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let act = Activity {
        id: ActivityId::new(100),
        user_id: UserId::from(1),
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
    };
    store.activity_create(act.clone()).await?;

    let modified = Activity {
        name: "Modified Ride".to_string(),
        ..act
    };
    let summary = modified.update(&test_session(), &mut store).await?;
    assert_eq!(summary.activities.len(), 1);
    assert_eq!(summary.activities[0].name, "Modified Ride");

    store.rollback().await?;
    Ok(())
}

/// Activity update replaces the data fields but keeps the stored row's
/// `utc_offset`, `device_name`, and `external_id` — the fields a frontend
/// round-trip can lose; one rule on both stores (#408 rule 1).
#[tokio::test]
async fn activity_update_preserves_fields() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    // A ride stored at 22:13:20+01:00, with device metadata.
    let act = Activity {
        id: ActivityId::new(100),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: "Morning Ride".to_string(),
        start: activity_start().to_offset(time::UtcOffset::from_whole_seconds(3600).unwrap()),
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: Some(300),
        energy: Some(1000),
        gear: None,
        device_name: Some("Garmin Edge".to_string()),
        external_id: Some("garmin_12345".to_string()),
    };
    store.activity_create(act.clone()).await?;

    // A client that lost the device data updates the name only, with the
    // start in a UTC representation.
    let lossy = Activity {
        name: "Modified Ride".to_string(),
        start: activity_start().to_offset(time::UtcOffset::UTC),
        device_name: None,
        external_id: None,
        ..act.clone()
    };
    let updated = store.activity_update(lossy).await?;

    // The rule: the data fields are replaced …
    assert_eq!(updated.name, "Modified Ride");
    assert_eq!(
        updated.start.unix_timestamp(),
        activity_start().unix_timestamp()
    );
    // … and the stored row keeps its offset, device name, and external id.
    assert_eq!(updated.start.offset().whole_seconds(), 3600);
    assert_eq!(updated.device_name.as_deref(), Some("Garmin Edge"));
    assert_eq!(updated.external_id.as_deref(), Some("garmin_12345"));

    // An update of a missing activity is a NotFound on both stores.
    let ghost = Activity {
        id: ActivityId::new(999),
        ..act
    };
    let err = store.activity_update(ghost).await;
    assert!(
        matches!(err, Err(tb_domain::Error::NotFound(_))),
        "updating a missing activity must be NotFound, got {err:?}"
    );

    store.rollback().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Activity: lookups
// ---------------------------------------------------------------------------

/// Listing activities in a time range: `begin` is included, `end` is
/// excluded — one rule on both stores (#408 rule 2).
#[tokio::test]
async fn activity_find_range_boundary() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let bike = create_part("Road Bike", "Trek", "Domane", BIKE, &mut store).await;
    let start = datetime!(2024-02-01 12:00 UTC);
    let act = Activity {
        id: ActivityId::new(100),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: "Boundary Ride".to_string(),
        start,
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: Some(300),
        energy: Some(1000),
        gear: Some(bike.id),
        device_name: None,
        external_id: None,
    };
    store.activity_create(act).await?;

    // A window that ends exactly at the activity's start: the boundary ride
    // is outside the range, because end is exclusive.
    let begin = start - time::Duration::minutes(30);
    let found = store
        .activities_find_by_gear_and_time(bike.id, begin, start)
        .await?;
    assert!(
        found.is_empty(),
        "a ride exactly at the range end must be excluded, got {found:?}"
    );

    // A window that begins exactly at the activity's start: begin is
    // inclusive.
    let found = store
        .activities_find_by_gear_and_time(bike.id, start, start + time::Duration::minutes(30))
        .await?;
    assert_eq!(
        found.len(),
        1,
        "a ride exactly at the range begin must be included"
    );
    assert_eq!(found[0].id, ActivityId::new(100));

    store.rollback().await?;
    Ok(())
}

/// The import lookup by user and time matches by the minute, not the
/// instant: the activity's local wall-clock minute (its start in the stored
/// offset) must equal the query's UTC wall-clock minute; one rule on both
/// stores (#408 rule 3). The same-minute duplicate case — two activities
/// whose local minutes coincide — is an open maintainer question, so this
/// test only covers the single-match and no-match cases.
#[tokio::test]
async fn activity_get_by_user_and_time() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let start = datetime!(2024-02-01 12:00:30 UTC);
    let act = Activity {
        id: ActivityId::new(100),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: "Minute Ride".to_string(),
        start,
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: Some(300),
        energy: Some(1000),
        gear: None,
        device_name: None,
        external_id: None,
    };
    store.activity_create(act.clone()).await?;

    // A different instant of the same minute: the minute match finds the
    // activity (a zero-offset ride's local minute is its UTC minute).
    let query = start + time::Duration::seconds(15);
    let found = store.get_by_user_and_time(UserId::from(1), query).await?;
    assert_eq!(found.id, ActivityId::new(100));

    // A different minute: no match.
    let other_minute = start + time::Duration::minutes(2);
    let err = store
        .get_by_user_and_time(UserId::from(1), other_minute)
        .await;
    assert!(
        matches!(err, Err(tb_domain::Error::NotFound(_))),
        "a different minute must not match, got {err:?}"
    );

    // A ride with a +02:00 offset matches only queries whose UTC wall clock
    // equals the ride's local minute (10:53), not its UTC minute (08:53).
    let local_start = OffsetDateTime::from_unix_timestamp(1706777630) // 08:53:50 UTC
        .unwrap()
        .to_offset(time::UtcOffset::from_whole_seconds(7200).unwrap());
    let act2 = Activity {
        id: ActivityId::new(101),
        start: local_start,
        ..act
    };
    store.activity_create(act2.clone()).await?;

    let query = OffsetDateTime::from_unix_timestamp(1706777630 + 7200 - 20).unwrap(); // 10:53:30 UTC — the ride's local minute
    let found = store.get_by_user_and_time(UserId::from(1), query).await?;
    assert_eq!(found.id, ActivityId::new(101));

    let next_minute = query + time::Duration::seconds(60); // 10:54:30 UTC
    let err = store
        .get_by_user_and_time(UserId::from(1), next_minute)
        .await;
    assert!(
        matches!(err, Err(tb_domain::Error::NotFound(_))),
        "a different minute must not match, got {err:?}"
    );

    store.rollback().await?;
    Ok(())
}

/// `get_all` returns the user's activities and `categories` derives the
/// unique gear types (mirrors the in-memory `activity_get_all` /
/// `activity_categories` tests).
#[tokio::test]
async fn activity_get_all_and_categories() -> tb_domain::TbResult<()> {
    let Some(Seam { _lock, mut store }) = seam().await else {
        return Ok(());
    };
    let acts = store.get_all(&UserId::from(1)).await?;
    assert_eq!(acts.len(), 3);
    let names: HashSet<&str> = acts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        ["Morning Ride", "Hill Repeats", "Recovery Spin"]
            .into_iter()
            .collect()
    );

    let categories = Activity::categories(&test_session(), &mut store).await?;
    assert_eq!(categories, HashSet::from([BIKE]));

    // A user without activities gets nothing.
    assert!(store.get_all(&UserId::from(99)).await?.is_empty());

    store.rollback().await?;
    Ok(())
}
