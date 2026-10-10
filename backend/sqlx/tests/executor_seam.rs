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

//! The per-user executor loop integration suite (issue #453).
//!
//! Drives the real executor loop — `tb_exec::run` over the real `DbPool`
//! (`TxnSource`/`Txn` implemented on the live `SqlxConn`, the seam the web
//! layer will use) — against real Postgres, with a stubbed
//! `StravaSession`: the loop's transaction lifecycle is what this suite
//! verifies, not the Strava API.
//!
//! What the four seam tests verify, end to end on the live adapter:
//!
//! - an **API write** travels the in-memory channel, runs in one live
//!   transaction, commits, resolves the route's oneshot, and pushes one
//!   stream frame — and the row is visible to a fresh connection afterwards;
//! - a **queued Strava `Stop`** past its expiry is dropped by the queue
//!   probe's housekeeping (committed by the probe) and the queue is empty
//!   afterwards;
//! - an **idle loop** with nothing queued and no streams attached reaps:
//!   `run` returns `Ok` (the web layer's respawn signal), not a hang;
//! - a **failed write** rolls its transaction back (no partial row),
//!   resolves the oneshot with the domain error, and the loop keeps going
//!   until it reaps.
//!
//! ## The scratch database this suite uses
//!
//! The suite is ignored by default (`-- --include-ignored` runs it) and
//! fails loudly whenever it cannot run, exactly like the store seam suite
//! (`store_seam.rs`): without `SCRATCH_DATABASE_URL` every test fails with
//! the no-URL message, and a scratch database that cannot be prepared fails
//! every test with the one root-cause string.
//!
//! One deliberate difference: the suite works on a **suffixed** scratch
//! database (`<scratch>_exec`), not the seam suite's — cargo runs test
//! binaries in parallel, and each suite's one-time setup force-drops the
//! database its URL names, so two suites on one URL would drop each
//! other's database out from under the run. The lifecycle is otherwise
//! shared through `common/scratch.rs` (the same canary, the same
//! one-time setup, the same lock).
//!
//! The per-test reset differs as well, and for a reason: the seam suite
//! rolls every test back, so a sequence reset is the whole cleanup; the
//! executor loop **commits** its messages, and a committed row outlives the
//! test. Each test therefore starts from a truncated-and-re-seeded scratch
//! database (`scratch::fresh_fixture`), the same state the unit tests see.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::*;

use std::time::Duration;

use serde::de::DeserializeOwned;
use tb_domain::test_support::{MemStore, part_type_ids::CHAIN};
use tb_domain::{ApiWrite, PartId, Session, ShopId, TbResult, UserId};
use tb_exec::{ApiWriteRequest, api_write_channel, run};
use tb_sqlx::DbPool;
use tb_strava::{StravaId, StravaSession, StravaStore};
use time::macros::datetime;
use tokio::sync::{MutexGuard, broadcast};

/// The database-name suffix this suite's scratch database carries: cargo
/// runs the `tb_sqlx` test binaries in parallel, and each suite's one-time
/// setup force-drops the database its URL names — a shared name would make
/// the suites drop each other's database out from under the run (see the
/// module docs).
const DB_SUFFIX: &str = "_exec";

/// The bound for awaiting the loop or one of its effects in a test: long
/// enough for a slow scratch database, short enough that a wedged loop
/// fails the test instead of hanging the suite.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The loop's idle window in the tests: short enough that the reap the
/// tests assert happens in a breath, long enough that a healthy loop never
/// reaps mid-message.
const IDLE_TIMEOUT: Duration = Duration::from_millis(300);

/// The in-memory suite's session identity: the fixture's one user and its
/// Strava id (the same pair the seam suite and the `tb_strava` tests use).
fn test_identity() -> (UserId, StravaId) {
    (UserId::from(1), StravaId::from(42))
}

/// The suite's `StravaSession`: the loop's session is the user's identity
/// plus the Strava API client, and the API client is what this suite
/// stubs. These tests never import from Strava (no activity event is
/// processed; the only queued event is a `Stop`, whose handling never
/// touches the API), so the client methods panic rather than pretend.
#[derive(Clone, Copy)]
struct FakeStrava {
    user: UserId,
    strava: StravaId,
    shop: Option<ShopId>,
    admin: bool,
}

impl FakeStrava {
    fn new() -> Self {
        let (user, strava) = test_identity();
        Self {
            user,
            strava,
            shop: None,
            admin: false,
        }
    }
}

impl Session for FakeStrava {
    fn user_id(&self) -> UserId {
        self.user
    }
    fn shop(&self) -> Option<ShopId> {
        self.shop
    }
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop;
        Ok(())
    }
    fn is_admin(&self) -> bool {
        self.admin
    }
}

#[async_trait::async_trait]
impl StravaSession for FakeStrava {
    fn strava_id(&self) -> StravaId {
        self.strava
    }
    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        _store: &mut impl StravaStore,
    ) -> TbResult<T> {
        panic!("the executor seam never imports from the Strava API (got {uri})")
    }
    async fn deauthorize(&mut self, _store: &mut impl StravaStore) -> TbResult<()> {
        panic!("the executor seam never deauthorizes a user")
    }
}

/// One test's hold on the scratch database: a pool for the test's runtime
/// plus the run's lock. The lock is held for the whole test (declared last
/// so it drops last): the tests serialize on the database, so a run is
/// deterministic regardless of the harness's test-parallelism, and the
/// committed rows one test left behind cannot reach another — the next
/// test's `fresh_fixture` runs under the same lock.
struct ExecSeam {
    /// Declared first so it is dropped first: the pool's connections return
    /// to the scratch database while the lock below is still held.
    pool: DbPool,
    /// Declared last so it is dropped last: the lock stays held while this
    /// test's pool is torn down.
    _lock: MutexGuard<'static, ()>,
}

/// Open one test's seam: take the run's lock, wait for this suite's
/// one-time setup (the suffixed scratch database), build this test's pool,
/// and reset the database to the pristine fixture (truncate + re-seed +
/// re-point the sequences — the committed-write model's per-test cleanup;
/// the seam suite's rolled-back model only needs the sequence reset).
async fn ready() -> ExecSeam {
    let _lock = LOCK.lock().await;
    let url = match suffixed_scratch_url(DB_SUFFIX) {
        Some(url) => url,
        None => panic!("{NO_SCRATCH_URL}"),
    };
    let setup = tokio::time::timeout(SETUP_STEP_TIMEOUT, SETUP.get_or_init(|| setup(&url)))
        .await
        .expect("the scratch database setup must finish in time");
    let Setup::Ready(_) = &setup else {
        panic!("the executor scratch database could not be prepared: {setup:?}");
    };
    let pool = pool(&url)
        .await
        .expect("the scratch database pool could not be built");
    fresh_fixture(&pool)
        .await
        .expect("re-seeding the fixture failed");
    ExecSeam { pool, _lock }
}

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

/// The named canary, mirroring the store seam suite's: whenever the suite
/// runs, this suite's scratch database must actually be preparable. With no
/// `SCRATCH_DATABASE_URL` — a local machine without a scratch database — it
/// fails with the no-URL message like the other tests; the `#[ignore]`
/// attribute keeps it out of plain runs.
#[tokio::test]
#[ignore]
async fn database_is_reachable() {
    canary(suffixed_scratch_url(DB_SUFFIX)).await;
}

// ---------------------------------------------------------------------------
// The seam tests
// ---------------------------------------------------------------------------

/// An API write end to end on the live adapter: the channel's request runs
/// in one live transaction, commits, resolves the route's oneshot with the
/// write's outcome (the `Summary`, the frame), pushes exactly one stream
/// frame, and the row is visible to a fresh connection afterwards. The idle
/// loop then reaps and `run` returns `Ok`.
#[tokio::test]
#[ignore]
async fn api_write_end_to_end() {
    let seam = ready().await;
    let (tx, rx) = api_write_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    let (request, reply) = ApiWriteRequest::new(ApiWrite::PartCreate {
        name: "Seam Chain".to_string(),
        vendor: "Shimano".to_string(),
        model: "CN-HG62".to_string(),
        what: CHAIN,
        purchase: datetime!(2024-01-01 00:00 UTC),
    });
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
        events,
        IDLE_TIMEOUT,
    ));

    // The route's oneshot resolves with the write's outcome; the part
    // write's `Summary` is the frame.
    let outcome = tokio::time::timeout(TEST_TIMEOUT, reply)
        .await
        .expect("the route's reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it")
        .expect("the part write must succeed");
    let summary = outcome.summary().clone();
    assert_eq!(summary.parts.len(), 1);
    let part = summary
        .parts
        .values()
        .next()
        .into_iter()
        .flatten()
        .next()
        .expect("the part");
    assert_eq!(part.name, "Seam Chain");
    assert_eq!(part.owner, test_identity().0);

    // One frame carrying the same summary reaches the stream.
    let frame = tokio::time::timeout(TEST_TIMEOUT, frame_rx.recv())
        .await
        .expect("the frame must arrive in time")
        .expect("the stream must stay open");
    assert_eq!(frame, summary);

    // A fresh connection sees the committed row.
    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM parts WHERE name = 'Seam Chain'")
        .fetch_one(&mut **conn)
        .await
        .expect("reading back the committed part");
    assert_eq!(n, 1, "the write's row must be committed");

    // Idle and unclaimed, the loop reaps and `run` returns `Ok`.
    drop(conn);
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");
}

/// A queued Strava `Stop` past its expiry (the rate-limit backoff) is
/// dropped by the queue probe's housekeeping — the probe commits its
/// deletion, the loop goes idle and reaps, and the queue is empty on a
/// fresh connection afterwards.
#[tokio::test]
#[ignore]
async fn strava_queue_drains_and_reclaims() {
    let seam = ready().await;
    // The user's Strava row and an expired `Stop` in the queue: the
    // `object_id` carries the backoff's deadline, long past.
    {
        let mut conn = seam.pool.begin().await.expect("a transaction");
        let (user, strava) = test_identity();
        sqlx::query(
            "INSERT INTO strava_users (id, tendabike_id, refresh_token) VALUES ($1, $2, 'tok')",
        )
        .bind(i32::from(strava))
        .bind(i32::from(user))
        .execute(&mut **conn)
        .await
        .expect("seeding the strava user");
        sqlx::query(
            "INSERT INTO strava_events \
             (object_type, object_id, aspect_type, updates, owner_id, subscription_id, event_time) \
             VALUES ('stop', 100, 'create', '{}', $1, 0, 100)",
        )
        .bind(i32::from(strava))
        .execute(&mut **conn)
        .await
        .expect("seeding the expired stop");
        conn.commit().await.expect("committing the seed");
    }

    // No API writes: the dropped sender closes the channel.
    let (_tx, rx) = api_write_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        run(
            seam.pool.clone(),
            FakeStrava::new(),
            rx,
            frames,
            events,
            IDLE_TIMEOUT,
        ),
    )
    .await
    .expect("the idle loop must reap in time");
    outcome.expect("a reaped loop returns Ok");

    // A fresh connection sees the drained queue: the probe's housekeeping
    // committed, so the expired stop is gone for good.
    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM strava_events")
        .fetch_one(&mut **conn)
        .await
        .expect("reading the queue");
    assert_eq!(n, 0, "the expired stop must be drained from the queue");
}

/// An idle loop — empty queue, closed channel, no streams attached —
/// reaps: `run` returns `Ok` promptly (the web layer respawns on it), not
/// a hang and not an error.
#[tokio::test]
#[ignore]
async fn idle_loop_reclaims() {
    let seam = ready().await;
    let (_tx, rx) = api_write_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    let started = std::time::Instant::now();
    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        run(
            seam.pool.clone(),
            FakeStrava::new(),
            rx,
            frames,
            events,
            IDLE_TIMEOUT,
        ),
    )
    .await
    .expect("an idle loop must reap in time");
    outcome.expect("a reaped loop returns Ok");
    // The reap came from the no-streams rule, well inside the idle window's
    // neighbourhood — certainly far under the test bound.
    assert!(
        started.elapsed() < TEST_TIMEOUT,
        "the reap must not take the whole test bound"
    );
}

/// A failed write: the domain op errors, the transaction rolls back (no
/// partial row), the oneshot carries the domain error to the route, and
/// the loop keeps going until it reaps.
#[tokio::test]
#[ignore]
async fn failed_write_rolls_back_and_reclaims() {
    let seam = ready().await;
    let fixture_parts = MemStore::prepopulated().snapshot().parts.len();

    let (tx, rx) = api_write_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    // Deleting a part that does not exist: `NotFound` from the domain.
    let (request, reply) = ApiWriteRequest::new(ApiWrite::PartDelete {
        id: PartId::from(999_999),
    });
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
        events,
        IDLE_TIMEOUT,
    ));

    // The route gets the domain error, not a 500 and not a hang.
    let err = tokio::time::timeout(TEST_TIMEOUT, reply)
        .await
        .expect("the route's reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it")
        .expect_err("a delete of a missing part must fail");
    assert!(
        matches!(err, tb_domain::Error::NotFound(_)),
        "the route must receive the domain error, got {err:?}"
    );

    // Nothing was written: the rollback left the fixture exactly as the
    // per-test re-seed left it.
    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM parts")
        .fetch_one(&mut **conn)
        .await
        .expect("reading the parts table");
    assert_eq!(
        n as usize, fixture_parts,
        "a failed write must leave no partial row behind"
    );
    drop(conn);

    // The loop survived the failure and reaped.
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");
}
