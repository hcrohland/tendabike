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

use std::collections::HashSet;
use std::time::Duration;

use sqlx::migrate::MigrateDatabase;
use tb_domain::test_support::{
    MemStore, StoreSnapshot,
    fixtures::{sample_purchase_date, test_session},
    part_type_ids,
};
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
// Plumbing: scratch-database lifecycle, serialization, one-time fixture seed
// ---------------------------------------------------------------------------

static LOCK: Mutex<()> = Mutex::const_new(());
static SETUP: OnceCell<Setup> = OnceCell::const_new();

/// The bound for each scratch-database create/drop step: long enough for a
/// slow disk, short enough that a blackholed endpoint fails in seconds.
const SETUP_STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// The one failure message shared by every test when the suite was explicitly
/// run (`-- --include-ignored`) but no scratch database is configured: the
/// run has nothing to verify against, so the tests fail loudly instead of
/// passing without verifying anything.
const NO_SCRATCH_URL: &str = "SCRATCH_DATABASE_URL is not set — the seam suite \
was explicitly run (-- --include-ignored) but has no scratch database to run \
against; set it to a disposable database URL and re-run";

/// The run's one-time scratch-database setup, shared by every test.
enum Setup {
    /// The fixture loaded and the sequences set; the high-water marks for
    /// the per-test sequence reset.
    Ready(FixtureMarks),
    /// The scratch database could not be prepared; the error is what every
    /// test fails with, and what `database_is_reachable` reports.
    Failed(String),
}

/// The fixture's id high-water marks, derived from the loaded snapshot so
/// the database's sequences continue where the fixture left off — the same
/// values the in-memory store derives for its next-id counters.
#[derive(Clone, Copy, Debug)]
struct FixtureMarks {
    /// The highest part id in the fixture; the next id handed out is one
    /// higher.
    parts: i32,
    /// The highest user id in the fixture; the next id handed out is one
    /// higher.
    users: i32,
}

/// The scratch database url from the environment or `.env`, if any. The URL
/// names the scratch database itself (see `setup`); the user it connects as
/// must hold createdb rights on that server.
///
/// This is the only database variable the suite *uses*. It also reads
/// `DATABASE_URL` and `DB_URL` only to refuse a collision: if the scratch
/// URL equals either, it panics — both point at the developer's working
/// database, and this suite force-drops and re-seeds whatever database it is
/// pointed at. Neither variable is ever used as a connection target.
fn scratch_url() -> Option<String> {
    let _ = dotenvy::dotenv();
    let Ok(url) = std::env::var("SCRATCH_DATABASE_URL") else {
        return None;
    };
    // Refuse a scratch URL that is one of the working databases: a run
    // against it would force-drop and re-seed real data. Exact string
    // equality on the raw values; an empty value counts as absent, so there
    // is nothing to collide with.
    let mut collisions = vec![];
    for var in ["DATABASE_URL", "DB_URL"] {
        if let Ok(other) = std::env::var(var)
            && !other.is_empty()
            && other == url
        {
            collisions.push(var);
        }
    }
    if !collisions.is_empty() {
        panic!(
            "SCRATCH_DATABASE_URL matches {} ({url}) — the seam suite \
             force-drops and re-seeds its scratch database; refusing to run \
             it against the working database. Set SCRATCH_DATABASE_URL to a \
             disposable database.",
            collisions.join(" and ")
        );
    }
    Some(url)
}

/// A fresh pool for this test's runtime, or the error string when the pool
/// cannot be built (the test then fails loudly with it).
///
/// Each `#[tokio::test]` runs on its own runtime, so a pool must never outlive
/// the runtime that created it: the pool's background tasks (connection
/// returns, maintenance) are spawned on that runtime and die with it, which
/// leaves the pool's slot accounting in a state where the next runtime's
/// `acquire` waits the full acquire timeout. Every test therefore opens its
/// own pool and drops it with the test.
async fn pool(url: &str) -> Result<tb_sqlx::DbPool, String> {
    tb_sqlx::DbPool::new(url)
        .await
        .map_err(|err| err.to_string())
}

/// Bound one scratch-database step so a blackholed endpoint fails in seconds
/// instead of hanging the run.
async fn bounded<T>(
    fut: impl std::future::Future<Output = Result<T, sqlx::Error>>,
) -> Result<T, String> {
    match tokio::time::timeout(SETUP_STEP_TIMEOUT, fut).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err(format!("timed out after {SETUP_STEP_TIMEOUT:?}")),
    }
}

/// Prepare the scratch database once per run: force-drop any database left
/// behind by a previous run (nothing is dropped at the end of a run), create
/// a fresh one via `MigrateDatabase`, run the migrations through the same
/// `DbPool::new` the app uses, and load the standard fixture.
///
/// Every step targets only the database named in `url` (the create/drop pair
/// from the server's maintenance database, the pool and the seed from the
/// scratch database itself): the suite never reaches any other database on
/// the machine.
async fn setup(url: &str) -> Setup {
    let exists = match bounded(sqlx::Postgres::database_exists(url)).await {
        Ok(exists) => exists,
        Err(err) => return Setup::Failed(format!("could not check the scratch database: {err}")),
    };
    if exists && let Err(err) = bounded(sqlx::Postgres::force_drop_database(url)).await {
        return Setup::Failed(format!(
            "could not drop the leftover scratch database: {err}"
        ));
    }
    if let Err(err) = bounded(sqlx::Postgres::create_database(url)).await {
        return Setup::Failed(format!("could not create the scratch database: {err}"));
    }
    match tb_sqlx::DbPool::new(url).await {
        Ok(pool) => match seed(&pool).await {
            Ok(marks) => Setup::Ready(marks),
            Err(err) => Setup::Failed(format!("could not seed the scratch database: {err}")),
        },
        Err(err) => Setup::Failed(format!("could not connect to the scratch database: {err}")),
    }
}

/// The suite's local mapping of `sqlx::Error` to the domain `Error`:
/// everything becomes `Error::DatabaseFailure`. The crate's `into_domain`
/// (`src/lib.rs`) also maps `RowNotFound` to `Error::NotFound`, but it is
/// crate-private and unreachable from this integration test — a separate
/// crate that can only build errors from the public `tb_domain::Error`
/// variants — and the raw statements this maps (`INSERT` in
/// `load_fixture`, `setval` in `seed` / `reset_sequences`) fail only for
/// database-failure reasons, so the narrower mapping suffices.
fn db_err(err: sqlx::Error) -> tb_domain::Error {
    tb_domain::Error::DatabaseFailure(err.into())
}

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

/// Load the standard prepopulated fixture (the same snapshot the in-memory
/// suite uses) into the scratch database — freshly created by `setup`, so it
/// is empty by construction and needs no truncate — then point the sequences
/// just past the fixture ids, returning the fixture's high-water marks for
/// the per-test sequence reset. The fixture is committed once; every test
/// afterwards works in its own transaction and rolls back.
async fn seed(pool: &tb_sqlx::DbPool) -> tb_domain::TbResult<FixtureMarks> {
    let mut tx = pool.begin().await?;

    let snap = MemStore::prepopulated().snapshot();

    load_fixture(&mut tx, &snap).await?;

    // The fixture rows use explicit ids; the sequences must continue just
    // past the fixture's highest ids.
    let marks = FixtureMarks {
        parts: snap
            .parts
            .iter()
            .map(|p| i32::from(p.id))
            .max()
            .unwrap_or(0),
        users: snap
            .users
            .iter()
            .map(|u| i32::from(u.id))
            .max()
            .unwrap_or(0),
    };
    sqlx::query("SELECT setval(pg_get_serial_sequence('parts', 'id'), $1)")
        .bind(marks.parts)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    sqlx::query("SELECT setval(pg_get_serial_sequence('users', 'id'), $1)")
        .bind(marks.users)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;

    tx.commit().await?;
    Ok(marks)
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

/// Point the sequences just past the fixture's highest ids inside the test
/// transaction, so freshly created parts/users get the same ids the
/// in-memory store hands out (max fixture id + 1). Sequence changes are
/// non-transactional, so each test resets them on entry.
async fn reset_sequences(
    tx: &mut tb_sqlx::SqlxConn<'_>,
    marks: &FixtureMarks,
) -> tb_domain::TbResult<()> {
    sqlx::query("SELECT setval(pg_get_serial_sequence('parts', 'id'), $1)")
        .bind(marks.parts)
        .execute(&mut ***tx)
        .await
        .map_err(db_err)?;
    sqlx::query("SELECT setval(pg_get_serial_sequence('users', 'id'), $1)")
        .bind(marks.users)
        .execute(&mut ***tx)
        .await
        .map_err(db_err)?;
    Ok(())
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
        // both stores — ascending `last_used` (#405); the fixture has many
        // parts that share a `last_used`, so within a tie the stores may
        // differ, and the assertion below checks the rule, not the exact
        // vector. Usages are built per part, so they follow the part
        // order. Activities come back ascending by start (#408). Only
        // attachments are still unordered in the database (a known
        // divergence).
        assert_eq!(summary.parts.len(), 17);
        for (earlier, later) in summary.parts.iter().zip(summary.parts.iter().skip(1)) {
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

        Ok(())
    })
    .await
}

/// Deleting an activity reverts the create: usage back at baseline and the
/// activity reported zeroed (mirrors
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
        assert_eq!(summary.activities[0].name, "Modified Ride");

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
