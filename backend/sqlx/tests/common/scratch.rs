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

//! The scratch-database plumbing shared by the `tb_sqlx` integration
//! suites: the store seam suite (`store_seam.rs`, issue #406/#411) and the
//! per-user executor suite (`executor_seam.rs`, issue #453).
//!
//! Each suite pulls this module in with `#[path = "common/scratch.rs"]`
//! (a test binary cannot pull a file outside its own directory tree any
//! other way) and compiles the whole file — the `pub` items are the seam
//! between the two suites, not a library.
//!
//! ## The lifecycle the plumbing implements
//!
//! - Every test runs against a **scratch database** named by
//!   `SCRATCH_DATABASE_URL` (a disposable, empty database; a CI service
//!   provides one, and `database_is_reachable` is the named canary that
//!   fails the job loudly when it cannot be prepared). The suite never
//!   connects to `DATABASE_URL` or `DB_URL`, which point at the developer's
//!   working database: it reads the two variables only to refuse a
//!   collision — a scratch URL that equals either fails the run instead of
//!   dropping real data.
//! - Setup is **one-time per run** (a `OnceCell`): force-drop any database
//!   left behind by a previous run, create a fresh one via
//!   `MigrateDatabase`, run the migrations, seed the fixture, and record
//!   the fixture's id high-water marks. A test that cannot prepare the
//!   database fails with the recorded error string; it never touches a
//!   database other than the scratch one.
//! - Tests **serialize on a mutex** around database work, and each opens
//!   its **own pool** (a pool must never outlive the runtime that created
//!   it: the pool's background tasks are spawned on that runtime and die
//!   with it, which wedges the underlying TCP connections).
//! - Determinism: [`reset_sequences`] puts the `users`/`parts` identity
//!   sequences just past the fixture ids inside each test's transaction
//!   (sequence changes are non-transactional, so each test resets them on
//!   entry), so the database allocates the same next ids the in-memory
//!   twin allocates and the `Summary` diff is deterministic regardless of
//!   test order.
//!
//! ## The two cleanup models
//!
//! The seam suite rolls every test back (its writes never commit), so the
//! sequence reset is the whole per-test cleanup. The executor suite cannot
//! do that — the loop *commits* its messages, and a committed row outlives
//! the test — so it resets with [`fresh_fixture`] (truncate every table,
//! re-seed the fixture, re-point the sequences) instead.

use sqlx::migrate::MigrateDatabase;
use std::time::Duration;
use tb_domain::test_support::{MemStore, StoreSnapshot};
use tb_domain::{ActivityStore, AttachmentStore, UsageStore};
use tb_sqlx::{DbPool, SqlxConn};
use tokio::sync::{Mutex, OnceCell};
use uuid::Uuid;

/// The one lock that serializes a suite's database work: at most one test
/// touches the scratch database at a time, so a run is deterministic
/// regardless of the harness's test-parallelism.
pub static LOCK: Mutex<()> = Mutex::const_new(());

/// The run's one-time scratch-database setup, shared by every test.
pub static SETUP: OnceCell<Setup> = OnceCell::const_new();

/// The bound for each scratch-database create/drop step: long enough for a
/// slow disk, short enough that a blackholed endpoint fails in seconds.
pub const SETUP_STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// The one failure message shared by every test when the suite was
/// explicitly run (`-- --include-ignored`) but no scratch database is
/// configured: the run has nothing to verify against, so the tests fail
/// loudly instead of passing without verifying anything.
pub const NO_SCRATCH_URL: &str = "SCRATCH_DATABASE_URL is not set — the seam suite \
was explicitly run (-- --include-ignored) but has no scratch database to run \
against; set it to a disposable database URL and re-run";

/// The run's one-time scratch-database setup, shared by every test.
#[derive(Debug)]
pub enum Setup {
    /// The fixture loaded and the sequences set; the high-water marks for
    /// the per-test sequence reset.
    ///
    /// `allow(dead_code)`: this file is compiled per suite, and the executor
    /// seam only matches `Ready(_)` (it re-seeds instead of re-pointing the
    /// sequences) — the seam suite reads the marks.
    #[allow(dead_code)]
    Ready(FixtureMarks),
    /// The scratch database could not be prepared; the error is what every
    /// test fails with, and what `database_is_reachable` reports.
    Failed(String),
}

/// The fixture's id high-water marks, derived from the loaded snapshot so
/// the database's sequences continue where the fixture left off — the same
/// values the in-memory store derives for its next-id counters.
#[derive(Clone, Copy, Debug)]
pub struct FixtureMarks {
    /// The highest part id in the fixture; the next id handed out is one
    /// higher.
    parts: i32,
    /// The highest user id in the fixture; the next id handed out is one
    /// higher.
    users: i32,
}

/// The scratch database url from the environment or `.env`, if any. The URL
/// names the scratch database itself (see [`setup`]); the user it connects
/// as must hold createdb rights on that server.
///
/// This is the only database variable the suites *use*. It also reads
/// `DATABASE_URL` and `DB_URL` only to refuse a collision: if the scratch
/// URL equals either, it panics — both point at the developer's working
/// database, and a suite force-drops and re-seeds whatever database it is
/// pointed at. Neither variable is ever used as a connection target.
pub fn scratch_url() -> Option<String> {
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
            "SCRATCH_DATABASE_URL matches {} ({url}) — the integration suite \
             force-drops and re-seeds its scratch database; refusing to run \
             it against the working database. Set SCRATCH_DATABASE_URL to a \
             disposable database.",
            collisions.join(" and ")
        );
    }
    Some(url)
}

/// The scratch database url for a suite that must not share its database
/// with another suite in the same `cargo test` run: cargo runs test
/// binaries in parallel, and a suite's one-time setup force-drops whatever
/// database its URL names — two suites pointed at the same URL would drop
/// the other's database out from under it. The run appends `suffix` to the
/// database name, so each suite gets its own disposable database (the
/// collision check in [`scratch_url`] still applies to the base URL, and
/// the suffixed name names no existing database: it is dropped at the start
/// of every run, like its base).
#[allow(dead_code)] // used by the executor suite; dead in the seam suite (the file is compiled per suite)
pub fn suffixed_scratch_url(suffix: &str) -> Option<String> {
    let url = scratch_url()?;
    let (prefix, name) = url.rsplit_once('/')?;
    Some(format!("{prefix}/{name}{suffix}"))
}

/// A fresh pool for this test's runtime, or the error string when the pool
/// cannot be built (the test then fails loudly with it).
///
/// Each `#[tokio::test]` runs on its own runtime, so a pool must never
/// outlive the runtime that created it: the pool's background tasks
/// (connection returns, maintenance) are spawned on that runtime and die
/// with it, which leaves the pool's slot accounting in a state where the
/// next runtime's `acquire` waits the full acquire timeout. Every test
/// therefore opens its own pool and drops it with the test.
pub async fn pool(url: &str) -> Result<DbPool, String> {
    tb_sqlx::DbPool::new(url)
        .await
        .map_err(|err| err.to_string())
}

/// Bound one scratch-database step so a blackholed endpoint fails in
/// seconds instead of hanging the run.
pub async fn bounded<T>(
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
pub async fn setup(url: &str) -> Setup {
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

/// The suites' local mapping of `sqlx::Error` to the domain `Error`:
/// everything becomes `Error::DatabaseFailure`. The crate's `into_domain`
/// (`src/lib.rs`) also maps `RowNotFound` to `Error::NotFound`, but it is
/// crate-private and unreachable from an integration test — a separate
/// crate that can only build errors from the public `tb_domain::Error`
/// variants — and the raw statements this maps (`INSERT` in
/// [`load_fixture`], `setval` in [`seed`]/[`reset_sequences`]) fail only
/// for database-failure reasons, so the narrower mapping suffices.
pub fn db_err(err: sqlx::Error) -> tb_domain::Error {
    tb_domain::Error::DatabaseFailure(err.into())
}

/// Load the standard prepopulated fixture (the same snapshot the in-memory
/// suite uses) into the scratch database — freshly created by [`setup`], so
/// it is empty by construction and needs no truncate — then point the
/// sequences just past the fixture ids, returning the fixture's
/// high-water marks for the per-test sequence reset. The fixture is
/// committed once; every test afterwards works in its own transaction and
/// rolls back.
pub async fn seed(pool: &DbPool) -> tb_domain::TbResult<FixtureMarks> {
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
pub async fn load_fixture(tx: &mut SqlxConn<'_>, snap: &StoreSnapshot) -> tb_domain::TbResult<()> {
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

/// Point the sequences just past the fixture's highest ids inside the open
/// transaction, so freshly created parts/users get the same ids the
/// in-memory store hands out (max fixture id + 1). Sequence changes are
/// non-transactional, so each test resets them on entry.
pub async fn reset_sequences(
    tx: &mut SqlxConn<'_>,
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

/// The executor suite's per-test reset (issue #453): truncate every table
/// in the schema and re-seed the standard fixture in one committed
/// transaction.
///
/// The seam suite rolls each test back (its writes never commit), so the
/// sequence reset is the whole per-test cleanup there; the executor suite
/// cannot — the loop *commits* its messages, and a committed row outlives
/// the test. Truncate + re-seed is that model's equivalent: every test
/// starts from exactly the prepopulated state the unit tests see, in the
/// same sequence positions, so the committed rows of one test never reach
/// the next and the diff is deterministic regardless of test order.
///
/// The `strava_*` tables are included in the truncate: the fixture has no
/// Strava rows, so an empty queue is exactly the fixture's state.
#[allow(dead_code)] // used by the executor suite; dead in the seam suite (the file is compiled per suite)
pub async fn fresh_fixture(pool: &DbPool) -> tb_domain::TbResult<()> {
    let mut tx = pool.begin().await?;
    // `RESTART IDENTITY` returns the identity sequences to their start
    // values; the `setval`s below then point them just past the re-loaded
    // fixture ids, exactly as the one-time seed leaves them. The migration
    // record table (sqlx 0.8 names it `_sqlx_migrations`) must not be
    // truncated: it would make the next `DbPool::new` re-apply migrations
    // that already ran, and a non-idempotent statement (the `CREATE
    // TRIGGER` without a `DROP` first) would fail the pool build.
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name NOT IN ('sqlx_migrations', '_sqlx_migrations')",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    sqlx::query(&format!("TRUNCATE {} RESTART IDENTITY", tables.join(", ")))
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    let snap = MemStore::prepopulated().snapshot();
    load_fixture(&mut tx, &snap).await?;
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
    reset_sequences(&mut tx, &marks).await?;
    tx.commit().await?;
    Ok(())
}

/// The body of a suite's `database_is_reachable` canary, shared by the
/// suites: under the lock, wait for the run's one-time setup of the
/// scratch database named by `url` — or fail the run loudly with the
/// suite's unified root-cause string (an unconfigured URL, or the setup's
/// error). The bound makes a blackholed endpoint fail in seconds instead
/// of burning the pool's acquire timeout per test.
#[allow(dead_code)] // used by the executor suite; dead in the seam suite (the file is compiled per suite)
pub async fn canary(url: Option<String>) {
    let Some(url) = url else {
        panic!("{NO_SCRATCH_URL}");
    };
    // Under the lock, like the other tests: the one-time setup runs exactly
    // once per run, and the canary is where its failure is reported. The
    // guard's only use of the lock is to hold it for the setup, hence the
    // underscore.
    let _lock = LOCK.lock().await;
    let outcome = tokio::time::timeout(SETUP_STEP_TIMEOUT, SETUP.get_or_init(|| setup(&url))).await;
    match outcome {
        Ok(Setup::Ready(_)) => {}
        Ok(Setup::Failed(err)) => panic!(
            "SCRATCH_DATABASE_URL is set ({url}) but the scratch database could \
             not be prepared: {err}"
        ),
        Err(_) => panic!(
            "SCRATCH_DATABASE_URL is set ({url}) but the scratch database could \
             not be prepared: it did not become ready within {SETUP_STEP_TIMEOUT:?} — \
             is the Postgres service running?"
        ),
    }
}
