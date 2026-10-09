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
//! The suite is deterministic: all tests run serialized against one freshly
//! created scratch database; a one-time seed loads the fixture into it, and
//! every test opens a transaction with the sequences reset just past the
//! fixture ids and rolls back, so each test starts from exactly the
//! prepopulated state.
//!
//! The suite manages its scratch database at `SCRATCH_DATABASE_URL` (falling
//! back to `.env`): on first use it force-drops any database left behind by
//! a previous run, creates a fresh one via `MigrateDatabase`, runs the
//! migrations, and seeds the fixture; nothing is dropped at the end of the
//! run, so the next run starts from the same clean slate. The URL's user
//! must hold createdb rights on that server. The suite never connects to
//! `DATABASE_URL` or `DB_URL`, which point at the developer's working
//! database: the original `DATABASE_URL`-based design seeded that database
//! in place, and a local run destroyed real data. It reads the two variables
//! only to refuse a collision — a scratch URL that equals either fails the
//! run instead of dropping real data.
//!
//! The suite is ignored by default: every test carries
//! `#[ignore]`, so a plain `cargo test --workspace` run (no database)
//! reports the 25 tests ignored and executes none of them,
//! and the existing in-memory suites stay green. It runs with
//! `cargo test -- --include-ignored` (the `--` matters: `--include-ignored`
//! is a libtest flag, not a cargo flag), and it fails loudly whenever it
//! cannot run: without `SCRATCH_DATABASE_URL`, every test fails with the
//! no-URL message, and a scratch database that cannot be prepared fails
//! every test with the one root-cause string. A test that verified nothing
//! never passes.
//!
//! The CI job that runs this suite against a real Postgres service is the
//! required `rust` job in `.github/workflows/test.yml` (issue #411): it
//! carries a `postgres:18` service, sets `SCRATCH_DATABASE_URL` to a scratch
//! database, and runs `cargo test --release -- --include-ignored`, so the
//! suite is part of the one required gate — the store adapters are unified
//! on the database's rules (#407-#410), and a red seam blocks the PR.
//! `database_is_reachable` is the named canary: it fails the job loudly when
//! the scratch database cannot be prepared. The seam contract — one rule per
//! operation, verified on both store adapters — is recorded in
//! `docs/agents/domain-flow.md`.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use tb_domain::test_support::{
    MemStore, StoreSnapshot,
    fixtures::{sample_purchase_date, test_session},
    part_type_ids,
};
use tb_domain::{
    ActTypeId, Activity, ActivityId, ActivityStore, ApiWrite, Attachment, AttachmentStore,
    MAX_TIME, OnboardingStatus, Part, PartId, PartNoteStore, PartStore, PartTypeId, ServicePlan,
    ServicePlanId, ShopStore, Summary, Usage, UsageId, UsageStore, UserId, UserStore,
    attach_assembly, detach_assembly, dispose_assembly, exec, round_time,
};
use time::{OffsetDateTime, macros::datetime};
use tokio::sync::MutexGuard;
use uuid::Uuid;

use part_type_ids::{BIKE, CHAIN, FRONT_WHEEL, TIRE};

#[path = "common/scratch.rs"]
mod scratch;
use scratch::*;

/// A test handle: one transaction on the shared test database. The lock is
/// held for the whole test so all tests in this suite run serialized; the
/// transaction is rolled back when the store is dropped.
struct Seam {
    /// Declared first so it is dropped first: the transaction rolls back
    /// while the lock below is still held.
    store: tb_sqlx::SqlxConn<'static>,
    /// Declared last so it is dropped last: the lock stays held while the
    /// test's transaction is rolled back on drop.
    _lock: MutexGuard<'static, ()>,
}

/// Open a fresh fixture transaction for a test, or the root-cause string of
/// why the seam cannot be opened (the test then fails loudly with it).
///
/// The lock first: the one-time scratch-database setup and the pool
/// connection (and its no-op migration run) are taken while the lock is
/// held, so tests never contend for the database.
///
/// The pool is local to this test: it is dropped when the `Seam` is built,
/// together with the test's runtime — the transaction keeps the pool alive
/// internally through its own `Arc` handle until it is rolled back.
async fn seam() -> Result<Seam, String> {
    let lock = LOCK.lock().await;
    let Some(url) = scratch_url() else {
        return Err(NO_SCRATCH_URL.to_string());
    };
    // The one-time setup runs inside the lock. A failure is recorded in the
    // cell for `database_is_reachable` to report; every test that finds it
    // failed fails loudly with the one root-cause string, so a broken
    // scratch database turns the whole suite red instead of passing with
    // nothing verified.
    let marks = match SETUP.get_or_init(|| setup(&url)).await {
        Setup::Ready(marks) => marks,
        Setup::Failed(err) => {
            return Err(format!("the scratch database could not be prepared: {err}"));
        }
    };
    let pool = match pool(&url).await {
        Ok(pool) => pool,
        Err(err) => {
            return Err(format!(
                "the scratch database could not be prepared: could not connect \
                 to the scratch database: {err}"
            ));
        }
    };
    let mut store = match pool.begin().await {
        Ok(store) => store,
        Err(err) => {
            return Err(format!(
                "the scratch database could not be prepared: could not begin a \
                 test transaction: {err}"
            ));
        }
    };
    if let Err(err) = reset_sequences(&mut store, marks).await {
        return Err(format!(
            "the scratch database could not be prepared: could not reset the \
             sequences: {err}"
        ));
    }
    Ok(Seam { store, _lock: lock })
    // `pool` is dropped here, with this test's runtime.
}

/// Run a test body against a fresh fixture transaction: opens the seam
/// (failing the test loudly with the root-cause string when it cannot — this
/// machine has no `SCRATCH_DATABASE_URL`, or the scratch database could not
/// be prepared), hands the store to the body by value, and returns the
/// body's result. The body never commits, so the transaction is rolled
/// back when the body's future drops; the `Seam`'s lock stays held until
/// that drop, so the suite still runs serialized — the destructure binds
/// `_lock` before `store`, and locals drop in reverse binding order, so the
/// lock outlives the store's rollback.
async fn with_seam<R>(f: impl FnOnce(tb_sqlx::SqlxConn<'static>) -> R) -> tb_domain::TbResult<()>
where
    R: std::future::Future<Output = tb_domain::TbResult<()>>,
{
    let Seam { _lock, store } = match seam().await {
        Ok(seam) => seam,
        Err(err) => panic!("{err}"),
    };
    f(store).await
}

// ---------------------------------------------------------------------------
// Sample data helpers, mirroring the in-memory suite's helpers
// ---------------------------------------------------------------------------

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

/// A test ride with the in-memory suite's sample metrics (one hour, 3500s,
/// 50km, 500m up, 300m down, 1000W), parameterized only by the fields the
/// call sites vary: the id, the name, the start instant, and the gear.
fn ride(id: i64, name: &str, start: OffsetDateTime, gear: Option<PartId>) -> Activity {
    Activity {
        id: ActivityId::new(id),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: name.to_string(),
        start,
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: Some(300),
        energy: Some(1000),
        gear,
        device_name: None,
        external_id: None,
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
// Availability
// ---------------------------------------------------------------------------

/// The named canary. The `rust` CI job sets `SCRATCH_DATABASE_URL`
/// unconditionally, so whenever this test runs the scratch database must
/// actually be preparable (dropped if a previous run left it behind, created,
/// migrated, and seeded): the failure is reported here with the suite's
/// unified root-cause string. The bound makes a blackholed endpoint fail in
/// seconds instead of burning the pool's 30-second acquire timeout per test.
///
/// With no `SCRATCH_DATABASE_URL` — a local machine without a scratch
/// database — it fails with the no-URL message like the other tests; the
/// `#[ignore]` attribute keeps it out of plain runs.
#[tokio::test]
#[ignore]
async fn database_is_reachable() {
    let Some(url) = scratch_url() else {
        panic!("{NO_SCRATCH_URL}");
    };
    // Under the lock, like the other tests: the one-time setup runs exactly
    // once per run, and this test is where its failure is reported. The
    // guard's only use of the lock is to hold it for the setup, hence the
    // underscore.
    let _lock = LOCK.lock().await;
    let outcome =
        tokio::time::timeout(Duration::from_secs(10), SETUP.get_or_init(|| setup(&url))).await;
    match outcome {
        Ok(Setup::Ready(_)) => {}
        Ok(Setup::Failed(err)) => panic!(
            "SCRATCH_DATABASE_URL is set ({url}) but the scratch database could \
             not be prepared: {err}"
        ),
        Err(_) => panic!(
            "SCRATCH_DATABASE_URL is set ({url}) but the scratch database could \
             not be prepared: it did not become ready within 10s — is the \
             Postgres service running?"
        ),
    }
    // `_lock` is dropped here, with this test's runtime.
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
#[ignore]
async fn fixture_roundtrip() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// User summary read
// ---------------------------------------------------------------------------

/// The user summary read returns the full fixture content.
#[tokio::test]
#[ignore]
async fn user_summary_read() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let summary = UserId::from(1).get_summary(None, &mut store).await?;

        // The fixture content. Parts come back in one unified order on
        // both stores — ascending `last_used` (#405) — but the summary map
        // is unordered, so the assertion below sorts the part values by
        // `last_used` and checks the rule on the sorted sequence.
        assert_eq!(summary.parts.len(), 17);
        let mut parts: Vec<_> = summary.parts.values().flatten().collect();
        parts.sort_by_key(|p| p.last_used);
        for (earlier, later) in parts.iter().zip(parts.iter().skip(1)) {
            assert!(
                earlier.last_used <= later.last_used,
                "parts not sorted by last_used"
            );
        }
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
            .values()
            .flatten()
            .find(|p| p.id == PartId::from(4))
            .unwrap();
        assert_eq!(chain.name, "Chain A");
        let att = summary
            .attachments
            .values()
            .flatten()
            .find(|a| a.a.part_id == PartId::from(4))
            .unwrap();
        assert_eq!(att.a.hook, BIKE);
        assert_eq!(att.a.detached, MAX_TIME);
        let usage = summary
            .usages
            .values()
            .flatten()
            .find(|u| u.id == chain.usage)
            .unwrap();
        assert_eq!(usage.time, 8025);
        assert_eq!(usage.count, 3);

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Attachment: attach
// ---------------------------------------------------------------------------

/// Attaching a part to a hook with nothing on it creates a single
/// still-attached row and bumps the part's last_used (mirrors
/// `attach_assembly_attaches_part_to_gear`).
#[tokio::test]
#[ignore]
async fn attach_new_part_to_empty_hook() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// The flat row model (ADR-0003): a tire mounted onto a front wheel that is
/// itself on the bike is stored against the top-level gear (mirrors
/// `attach_assembly_resolves_mounted_gear_to_top_level`).
#[tokio::test]
#[ignore]
async fn attach_resolves_mounted_gear_to_top_level() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// Re-attaching a part at the time its row started deletes and re-creates
/// the row: exactly one still-attached row remains (mirrors
/// `attach_assembly_auto_detaches_and_reattaches_same_part`).
#[tokio::test]
#[ignore]
async fn attach_reattach_at_own_time() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// Attaching a part detaches the different part already occupying the hook
/// (mirrors `attach_assembly_detaches_predecessor_on_gear`).
#[tokio::test]
#[ignore]
async fn attach_detaches_predecessor() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Attachment: merge and delete (rules unified in #407)
// ---------------------------------------------------------------------------

/// Re-attaching a part at the time its row ended continues the same row —
/// the adjacent-merge (unified rule #407, one rule on both stores).
#[tokio::test]
#[ignore]
async fn attach_merge_adjacent_with_previous() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// An attachment delete addresses its row by part + attach time — the
/// database's key — so a stale gear field in the argument does not change
/// which row is deleted (unified rule #407).
#[tokio::test]
#[ignore]
async fn attachment_delete_identity() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// Finding the successor of an attachment: another part, of the same type,
/// at the same hook, attached later — the part's own later rows never count
/// (unified rule #407).
#[tokio::test]
#[ignore]
async fn attachment_find_successor() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// Finding the attachment already attached to a part: the row of this part
/// at this gear and hook that ended exactly at the query time — the
/// adjacent-merge trigger (unified rule #407).
#[tokio::test]
#[ignore]
async fn attachment_find_part_attached_already() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// `find_part_attached_already` for a part that was never attached is
/// `None` on both stores (no divergence).
#[tokio::test]
#[ignore]
async fn find_attached_already_never_attached() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

        let found = store
            .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, attachment_time())
            .await?;
        assert!(found.is_none());

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Attachment: detach and dispose
// ---------------------------------------------------------------------------

/// Detaching a fixture part re-cuts the attachment at the detach time and
/// re-derives its usage from the activities inside the new window (mirrors
/// `detach_assembly_recalculates_usage_excluding_activity_after_detach`).
#[tokio::test]
#[ignore]
async fn detach_recuts_and_recalculates_usage() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// Detaching a part that is not attached is a not-found error (mirrors
/// `detach_assembly_api_returns_error_if_not_attached`).
#[tokio::test]
#[ignore]
async fn detach_not_attached_is_not_found() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let session = test_session();
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

        let result =
            detach_assembly(&session, chain.id, attachment_time(), false, &mut store).await;
        assert!(
            matches!(&result, Err(tb_domain::Error::NotFound(_))),
            "detaching a never-attached part must fail, got {result:?}"
        );

        Ok(())
    })
    .await
}

/// Disposing a loose part sets its disposed timestamp (mirrors
/// `dispose_assembly_disposes_part`).
#[tokio::test]
#[ignore]
async fn dispose_part() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let session = test_session();
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

        // Dispose at later_time (no current attachment).
        let _ = dispose_assembly(&session, chain.id, later_time(), false, &mut store).await?;

        let part = store.partid_get_part(chain.id).await?;
        assert_eq!(part.disposed_at, Some(later_time()));

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Activity: upsert, update, delete
// ---------------------------------------------------------------------------

/// The activity id is a primary key on both stores (issue #405): the
/// INSERT hits the key on a duplicate id and maps to
/// `Error::DatabaseFailure` — the in-memory store applies the same rule.
/// The in-memory twin is `activity_create_rejects_duplicate_id` in
/// `mem_activity.rs`.
#[tokio::test]
#[ignore]
async fn activity_create_rejects_duplicate_id() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let ride = ride(100, "Ride", activity_start(), None);
        store.activity_create(ride.clone()).await?;

        // The first ride is stored, and a different id still succeeds.
        let stored = store
            .activity_read_by_id(ActivityId::new(100))
            .await?
            .expect("the first ride is stored");
        assert_eq!(stored.id, ActivityId::new(100));
        let mut other = ride.clone();
        other.id = ActivityId::new(101);
        store.activity_create(other).await?;

        // The duplicate create comes last: a failed statement aborts the
        // Postgres transaction, so nothing may follow it inside this test.
        let err = store
            .activity_create(ride.clone())
            .await
            .expect_err("a duplicate activity id must fail");
        assert!(
            matches!(err, tb_domain::Error::DatabaseFailure(_)),
            "a duplicate activity id must be a DatabaseFailure, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// Creating a new activity for the prepopulated bike accounts for the bike
/// and every part attached to it (mirrors
/// `activity_upsert_creates_new_accounts_bike_and_attached_parts`).
#[tokio::test]
#[ignore]
async fn activity_upsert_creates_and_accounts() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = PartId::from(1);
        let act = Activity {
            time: Some(1000),
            distance: Some(10000),
            climb: Some(100),
            descend: None,
            energy: Some(200),
            ..ride(100, "New Ride", activity_start(), Some(bike))
        };

        let summary = act.clone().upsert(&test_session(), &mut store).await?;

        // The new activity is reported with the bike as gear.
        assert_eq!(summary.activities, HashMap::from([(act.id, Some(act))]));

        // The bike and all attached parts: front wheel, rear wheel, chain, tires.
        let part_ids: HashSet<PartId> = summary.parts.values().flatten().map(|p| p.id).collect();
        assert_eq!(
            part_ids,
            [1, 2, 3, 4, 5, 6].into_iter().map(PartId::from).collect()
        );
        // The bike's last_used is bumped to the activity start.
        let bike_part = summary
            .parts
            .values()
            .flatten()
            .find(|p| p.id == bike)
            .unwrap();
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
        for (id, u) in &summary.usages {
            expected.id = *id;
            assert_eq!(u.as_ref().unwrap(), &expected, "unexpected usage for {id}");
        }

        // The updates are persisted in the store.
        let bike_usage = store.partid_get_part(bike).await?.usage;
        let stored = UsageStore::get(&mut store, bike_usage)
            .await?
            .expect("the bike usage row exists");
        assert_eq!(stored.time, 9025);
        assert_eq!(stored.distance, 135000);
        assert_eq!(stored.count, 4);

        Ok(())
    })
    .await
}

/// Deleting an activity reverts the create: usage back at baseline and the
/// activity reported as a None tombstone (mirrors
/// `activity_delete_reverts_bike_and_attached_part_usage`).
#[tokio::test]
#[ignore]
async fn activity_delete_reverts_usage() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = PartId::from(1);
        let act = Activity {
            time: Some(1000),
            distance: Some(10000),
            climb: Some(100),
            descend: None,
            energy: Some(200),
            ..ride(100, "New Ride", activity_start(), Some(bike))
        };

        // Create the activity so its usage is accounted, then delete it.
        act.clone().upsert(&test_session(), &mut store).await?;
        let summary = ActivityId::new(100)
            .delete(&test_session(), &mut store)
            .await?;

        // The deleted activity is reported as a None tombstone.
        assert_eq!(summary.activities, HashMap::from([(act.id, None)]));

        // The bike and all attached parts are affected again.
        let part_ids: HashSet<PartId> = summary.parts.values().flatten().map(|p| p.id).collect();
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
        for (id, u) in &summary.usages {
            expected.id = *id;
            assert_eq!(u.as_ref().unwrap(), &expected, "unexpected usage for {id}");
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

        Ok(())
    })
    .await
}

/// Updating an activity returns a summary containing it (mirrors
/// `activity_update_returns_summary`). The ride is created under a fresh id
/// on both stores for the same reason: the fixture's activity id is already
/// taken, and both stores reject a duplicate id — the database's primary
/// key, mirrored in the in-memory store (issue #405).
#[tokio::test]
#[ignore]
async fn activity_update_returns_summary() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let act = Activity {
            device_name: Some("Garmin Edge".to_string()),
            external_id: Some("garmin_12345".to_string()),
            ..ride(100, "Morning Ride", activity_start(), None)
        };
        store.activity_create(act.clone()).await?;

        let modified = Activity {
            name: "Modified Ride".to_string(),
            ..act
        };
        let summary = modified.update(&test_session(), &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        assert_eq!(
            summary.activities[&ActivityId::new(100)]
                .as_ref()
                .unwrap()
                .name,
            "Modified Ride"
        );

        Ok(())
    })
    .await
}

/// Activity update replaces the data fields but keeps the stored row's
/// `utc_offset`, `device_name`, and `external_id` — the fields a frontend
/// round-trip can lose; one rule on both stores (#408 rule 1).
#[tokio::test]
#[ignore]
async fn activity_update_preserves_fields() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        // A ride stored at 22:13:20+01:00, with device metadata.
        let act = Activity {
            device_name: Some("Garmin Edge".to_string()),
            external_id: Some("garmin_12345".to_string()),
            ..ride(
                100,
                "Morning Ride",
                activity_start().to_offset(time::UtcOffset::from_whole_seconds(3600).unwrap()),
                None,
            )
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

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Activity: lookups
// ---------------------------------------------------------------------------

/// Listing activities in a time range: `begin` is included, `end` is
/// excluded — one rule on both stores (#408 rule 2).
#[tokio::test]
#[ignore]
async fn activity_find_range_boundary() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = create_part("Road Bike", "Trek", "Domane", BIKE, &mut store).await;
        let start = datetime!(2024-02-01 12:00 UTC);
        let act = ride(100, "Boundary Ride", start, Some(bike.id));
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

        Ok(())
    })
    .await
}

/// The import lookup by user and time matches by the minute, not the
/// instant: the activity's local wall-clock minute (its start in the stored
/// offset) must equal the query's UTC wall-clock minute; one rule on both
/// stores (#408 rule 3). Zero matches is NotFound, exactly one returns the
/// activity, and the maintainer-confirmed duplicate rule applies: two or
/// more activities in the same minute is an Error::Ambiguous.
#[tokio::test]
#[ignore]
async fn activity_get_by_user_and_time() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let start = datetime!(2024-02-01 12:00:30 UTC);
        let act = ride(100, "Minute Ride", start, None);
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

        // The confirmed duplicate rule: a second ride in the first ride's minute
        // makes the lookup ambiguous — an error, not a silent first-match, so a
        // conflicting CSV row lands in the bad list.
        let dup = Activity {
            id: ActivityId::new(102),
            start: start + time::Duration::seconds(15), // 12:00:45 — the first ride's minute
            ..act2.clone()
        };
        store.activity_create(dup).await?;
        let err = store
            .get_by_user_and_time(UserId::from(1), start + time::Duration::seconds(20))
            .await;
        assert!(
            matches!(err, Err(tb_domain::Error::Ambiguous(_))),
            "two same-minute rides must return Error::Ambiguous, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// `get_all` returns the user's activities and `categories` derives the
/// unique gear types (mirrors the in-memory `activity_get_all` /
/// `activity_categories` tests).
#[tokio::test]
#[ignore]
async fn activity_get_all_and_categories() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
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

        Ok(())
    })
    .await
}

/// `get_all` returns the user's activities in ascending start instant —
/// the database's `ORDER BY start` is the one rule on both stores (issue
/// #405). The later ride is created first; the listing must not follow
/// creation order.
#[tokio::test]
#[ignore]
async fn activity_get_all_orders_by_start() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = PartId::from(1);

        // The later ride is created first.
        let later = ride(
            100,
            "Later Ride",
            activity_start() + time::Duration::hours(1),
            Some(bike),
        );
        store.activity_create(later).await?;

        // The earlier ride is created second.
        let earlier = ride(101, "Earlier Ride", activity_start(), Some(bike));
        store.activity_create(earlier).await?;

        // The three fixture rides (May 2023) precede the two created rides
        // (November 2023), and the created rides come back in start order,
        // not creation order.
        let acts = store.get_all(&UserId::from(1)).await?;
        let names: Vec<&str> = acts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Morning Ride",
                "Hill Repeats",
                "Recovery Spin",
                "Earlier Ride",
                "Later Ride"
            ]
        );

        Ok(())
    })
    .await
}

/// A production read re-expresses the stored start in the stored offset
/// rounded to the nearest 30 minutes — the instant never moves, only the
/// label does (the one rule documented on `Activity`, issue #409; the
/// in-memory twin is `activity_offsets_normalized_to_30_minutes` in
/// `mem_activity.rs`). Every other ride in this suite sits exactly on a
/// 30-minute boundary (UTC, +01:00, +02:00), where the rounding is a
/// no-op, so the rule is only visible with an off-boundary offset: a
/// +00:20 start comes back as +00:30.
#[tokio::test]
#[ignore]
async fn activity_read_rounds_offset_to_30_minutes() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        // The sample start expressed with a +00:20 offset — off-boundary.
        let start = activity_start().to_offset(time::UtcOffset::from_whole_seconds(1200).unwrap());
        let ride = ride(100, "Offset Ride", start, None);
        store.activity_create(ride).await?;

        let read = store
            .activity_read_by_id(ActivityId::new(100))
            .await?
            .expect("the ride is stored");

        // The rule: the offset is rounded to the nearest 30 minutes …
        assert_eq!(
            read.start.offset().whole_seconds(),
            1800,
            "a +00:20 offset must round to +00:30"
        );
        // … and the instant never moves.
        assert_eq!(read.start.unix_timestamp(), start.unix_timestamp());

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// ApiWrite dispatch (issue #452)
// ---------------------------------------------------------------------------
//
// The same representative set of writes as the in-memory dispatch suite in
// `tb_domain`, driven through the real database: the dispatch applies the
// same domain operations the handlers use, and the write contract — a
// `Summary` of everything touched, tombstones for deletes, an empty summary
// for the non-Summary kinds — holds on the source of truth.

/// The dispatch applies part writes: create returns the part, change returns
/// the part, delete reports the tombstone, and a missing part is NotFound.
#[tokio::test]
#[ignore]
async fn apiwrite_part_create_change_delete() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let summary = exec(
            ApiWrite::PartCreate {
                name: "New Chain".to_string(),
                vendor: "Shimano".to_string(),
                model: "CN-HG62".to_string(),
                what: CHAIN,
                purchase: sample_purchase_date(),
            },
            &mut session,
            &mut store,
        )
        .await?;
        let id = *summary.parts.keys().next().unwrap();
        assert_eq!(summary.parts[&id].as_ref().unwrap().name, "New Chain");

        let summary = exec(
            ApiWrite::PartChange {
                id,
                name: "Renamed".to_string(),
                vendor: "New Vendor".to_string(),
                model: "New Model".to_string(),
                purchase: sample_purchase_date(),
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.parts[&id].as_ref().unwrap().name, "Renamed");

        let summary = exec(ApiWrite::PartDelete { id }, &mut session, &mut store).await?;
        assert_eq!(summary.parts, HashMap::from([(id, None)]));
        assert!(store.partid_get_part(id).await.is_err());

        // The failed statement aborts the Postgres transaction, so the
        // missing-part error comes last.
        let err = exec(
            ApiWrite::PartDelete {
                id: PartId::from(9999),
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, tb_domain::Error::NotFound(_)),
            "deleting a missing part must be NotFound, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// The dispatch drives the attachment rules through the same ops the
/// handlers use: attach creates the row and bumps the part, detach re-cuts it.
#[tokio::test]
#[ignore]
async fn apiwrite_attach_detach() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();
        let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;
        let time = attachment_time();

        let summary = exec(
            ApiWrite::AttachmentAttach {
                part: chain.id,
                time,
                gear: bike.id,
                hook: BIKE,
                all: false,
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert!(!summary.parts.is_empty());
        let att = store
            .attachment_get_by_part_and_time(chain.id, time)
            .await?
            .expect("the chain is attached");
        assert_eq!(att.gear, bike.id);
        assert_eq!(att.detached, MAX_TIME);

        let _ = exec(
            ApiWrite::AttachmentDetach {
                part: chain.id,
                time,
                all: false,
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert!(
            store
                .attachment_get_by_part_and_time(chain.id, time)
                .await?
                .is_none(),
            "the detach cuts the row"
        );

        Ok(())
    })
    .await
}

/// The dispatch applies activity writes: update reports the activity, delete
/// reports the tombstone and reverts the usage accounting.
#[tokio::test]
#[ignore]
async fn apiwrite_activity_update_and_delete() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        // Update the first fixture ride through the dispatch.
        let mut act = store
            .activity_read_by_id(ActivityId::new(1))
            .await?
            .expect("the fixture activity");
        let id = act.id;
        act.name = "Renamed Ride".to_string();
        let summary = exec(
            ApiWrite::ActivityUpdate { id, activity: act },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(
            summary.activities[&id].as_ref().unwrap().name,
            "Renamed Ride"
        );

        // A path/body id mismatch is the handler's guard, surfaced as BadRequest.
        let err = exec(
            ApiWrite::ActivityUpdate {
                id: ActivityId::new(2),
                activity: ride(100, "Ride", activity_start(), None),
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, tb_domain::Error::BadRequest(_)),
            "a path/body id mismatch must be BadRequest, got {err:?}"
        );

        // Delete a created ride: the tombstone and the usage revert.
        let ride = ride(100, "New Ride", activity_start(), Some(PartId::from(1)));
        ride.clone().upsert(&session, &mut store).await?;
        let summary = exec(
            ApiWrite::ActivityDelete {
                id: ActivityId::new(100),
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(
            summary.activities,
            HashMap::from([(ActivityId::new(100), None)])
        );
        assert!(
            store
                .activity_read_by_id(ActivityId::new(100))
                .await?
                .is_none(),
            "the ride is deleted"
        );

        Ok(())
    })
    .await
}

/// The dispatch runs the CSV descend import the same way the handler does:
/// matched rows come back in the summary, the others in the store unchanged.
#[tokio::test]
#[ignore]
async fn apiwrite_activity_descend() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let csv = "Date,Title,Total Descent\n2023-05-18 22:13:20,Morning Ride,900\n";
        let summary = exec(
            ApiWrite::ActivityDescend {
                data: csv.to_string(),
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(
            summary.activities[&ActivityId::new(1)]
                .as_ref()
                .unwrap()
                .descend,
            Some(900)
        );
        let stored = store
            .activity_read_by_id(ActivityId::new(1))
            .await?
            .unwrap();
        assert_eq!(stored.descend, Some(900));

        Ok(())
    })
    .await
}

/// The dispatch applies part-note writes: create returns the note, delete
/// reports the tombstone.
#[tokio::test]
#[ignore]
async fn apiwrite_partnote_create_and_delete() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();
        let part = PartId::from(13);

        let summary = exec(
            ApiWrite::PartNoteCreateText {
                part,
                name: "Check the tension".to_string(),
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.part_notes.len(), 1);
        let note = summary.part_notes.values().flatten().next().unwrap();
        assert_eq!(note.name, "Check the tension");
        assert_eq!(note.part, part);

        let summary = exec(
            ApiWrite::PartNoteDelete { id: note.id },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.part_notes, HashMap::from([(note.id, None)]));
        assert!(
            store.partnote_get(note.id).await.is_err(),
            "the note is deleted"
        );

        Ok(())
    })
    .await
}

/// The dispatch applies service writes: create accounts the part usage and
/// returns the service plus its usage, delete reports the tombstone.
#[tokio::test]
#[ignore]
async fn apiwrite_service_create_and_delete() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let summary = exec(
            ApiWrite::ServiceCreate {
                part: PartId::from(13),
                time: datetime!(2024-06-15 10:00 UTC),
                name: "Chain Service".to_string(),
                notes: "Old chain".to_string(),
                plans: vec![],
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.services.len(), 1);
        assert_eq!(summary.usages.len(), 1);
        let service = summary.services.values().flatten().next().unwrap();
        assert_eq!(service.name, "Chain Service");

        let summary = exec(
            ApiWrite::ServiceDelete { id: service.id },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.services, HashMap::from([(service.id, None)]));

        Ok(())
    })
    .await
}

/// The dispatch applies plan writes: create returns the plan, delete reports
/// the plan tombstone (and the unlinked services, of which there are none
/// here).
#[tokio::test]
#[ignore]
async fn apiwrite_serviceplan_create_and_delete() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let plan = ServicePlan {
            id: ServicePlanId::from(
                Uuid::from_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(),
            ),
            part: Some(PartId::from(13)),
            what: CHAIN,
            hook: None,
            name: "Chain Every 1000km".to_string(),
            days: None,
            hours: None,
            km: Some(1000),
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let summary = exec(
            ApiWrite::ServicePlanCreate { plan },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.plans.len(), 1);
        let id = *summary.plans.keys().next().unwrap();

        let summary = exec(ApiWrite::ServicePlanDelete { id }, &mut session, &mut store).await?;
        assert_eq!(summary.plans, HashMap::from([(id, None)]));

        Ok(())
    })
    .await
}

/// The dispatch applies shop writes: create returns the shop, register and
/// unregister report the part (the route needs the owner's own subscription
/// first), delete reports the tombstone.
#[tokio::test]
#[ignore]
async fn apiwrite_shop_crud() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let summary = exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.shops.len(), 1);
        let shop_id = *summary.shops.keys().next().unwrap();

        // The register route's checkuser requires the owner's subscription;
        // auto-approve activates it.
        let _ = exec(
            ApiWrite::ShopSubscriptionCreate {
                shop: shop_id,
                message: None,
            },
            &mut session,
            &mut store,
        )
        .await?;

        let part = PartId::from(13); // loose spare, owned by user 1
        let summary = exec(
            ApiWrite::ShopRegisterPart {
                shop: shop_id,
                part,
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.parts[&part].as_ref().unwrap().shop, Some(shop_id));

        let summary = exec(
            ApiWrite::ShopUnregisterPart {
                shop: shop_id,
                part,
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.parts[&part].as_ref().unwrap().shop, None);

        let summary = exec(
            ApiWrite::ShopDelete { id: shop_id },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary.shops, HashMap::from([(shop_id, None)]));

        Ok(())
    })
    .await
}

/// The dispatch applies subscription writes: a subscription is not a kind of
/// the `Summary`, so the writes report an empty summary and the side effect
/// lands in the store.
#[tokio::test]
#[ignore]
async fn apiwrite_subscription_create_and_cancel() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let summary = exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &mut session,
            &mut store,
        )
        .await?;
        let shop_id = *summary.shops.keys().next().unwrap();

        let summary = exec(
            ApiWrite::ShopSubscriptionCreate {
                shop: shop_id,
                message: Some("Please approve".to_string()),
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary, Summary::default());
        let sub = store
            .subscription_find_active(shop_id, UserId::from(1))
            .await?
            .expect("auto-approve activates the subscription");
        assert_eq!(sub.status, tb_domain::SubscriptionStatus::Active);

        let summary = exec(
            ApiWrite::ShopSubscriptionCancel { id: sub.id },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary, Summary::default());
        assert!(
            store.subscription_get(sub.id).await.is_err(),
            "the subscription is deleted"
        );

        Ok(())
    })
    .await
}

/// The dispatch does the onboarding domain part only — the status guard with
/// the handler's message, then the status update; the Strava-side sync event
/// is the cutover ticket's concern (issue #457). A second trigger is
/// rejected with the handler's message.
#[tokio::test]
#[ignore]
async fn apiwrite_onboarding_sync() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let summary = exec(
            ApiWrite::UserOnboardingSync { time: 0 },
            &mut session,
            &mut store,
        )
        .await?;
        assert_eq!(summary, Summary::default());
        let user = UserStore::get(&mut store, UserId::from(1)).await?;
        assert_eq!(user.onboarding_status, OnboardingStatus::Completed);

        // The guard: a second trigger is a BadRequest with the handler's
        // message. The failed statement aborts the Postgres transaction, so
        // it comes last.
        let err = exec(
            ApiWrite::UserOnboardingSync { time: 0 },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(&err, tb_domain::Error::BadRequest(msg) if msg == "Initial sync already triggered"),
            "a second sync must be the handler's BadRequest, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// The onboarding postpone guard and status update, the domain part the
/// handler runs; a second postpone is rejected with the handler's message.
#[tokio::test]
#[ignore]
async fn apiwrite_onboarding_postpone() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let mut session = test_session();

        let summary = exec(ApiWrite::UserOnboardingPostpone, &mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        let user = UserStore::get(&mut store, UserId::from(1)).await?;
        assert_eq!(
            user.onboarding_status,
            OnboardingStatus::InitialSyncPostponed
        );

        // The guard: not pending anymore.
        let err = exec(ApiWrite::UserOnboardingPostpone, &mut session, &mut store)
            .await
            .unwrap_err();
        assert!(
            matches!(
                &err,
                tb_domain::Error::BadRequest(msg)
                    if msg == "Initial sync already completed or postponed"
            ),
            "a second postpone must be the handler's BadRequest, got {err:?}"
        );

        Ok(())
    })
    .await
}
