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
//! layer will use) — against real Postgres, with a stubbed `Session`: the
//! loop's transaction lifecycle is what this suite verifies.
//!
//! What the eight seam tests verify, end to end on the live adapter:
//!
//! - an **API write** travels the in-memory channel, runs in one live
//!   transaction, commits, resolves the route's oneshot, and pushes one
//!   stream frame — and the row is visible to a fresh connection afterwards;
//! - a **read as a message** travels the in-memory channel, runs in one live
//!   loop-owned transaction (read-only within it by spec #558 §3.6),
//!   resolves the route's oneshot with the `ReadOutcome`, and pushes **no**
//!   stream frame;
//! - a **read enqueued after a write** sees that write (FIFO total order
//!   per user, spec #558 §3.2) — the write's committed row is in the
//!   read's `ReadOutcome`;
//! - a **hydration write** (`Request::Gear`) runs `update_gear` in one live
//!   loop-owned transaction with the loop's session and pushes a frame —
//!   the union of the parts it created — and a `User` read enqueued after
//!   it (FIFO) sees the committed hydration in its snapshot (spec #558
//!   §4); the common case (no new gear) pushes **no** frame and leaves the
//!   snapshot unchanged;
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

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tb_domain::test_support::{MemStore, part_type_ids::CHAIN, sole_live};
use tb_domain::{ApiRead, ApiWrite, PartId, ReadOutcome, Session, ShopId, Summary, UserId};
use tb_exec::{
    DetachedBegin, ExecutorStatus, FramePusher, Registry, Request, ServiceApi, request_channel, run,
};
use tb_sqlx::DbPool;
use tb_strava::{
    StravaId,
    event::InEvent,
    test_support::{canned_strava, seed_token_cache},
};
use time::macros::datetime;
use tokio::sync::{MutexGuard, Notify, broadcast};

/// The database-name suffix this suite's scratch database carries: cargo
/// runs the `tb_sqlx` test binaries in parallel, and each suite's one-time
/// setup force-drops the database its URL names — a shared name would make
/// the suites drop each other's database out from under the run (see the
/// module docs).
const DB_SUFFIX: &str = "_exec";

/// A no-op `FramePusher`: the seam tests' sessions all have `shop() == None`,
/// so the executor takes the user-context path and never calls the pusher.
struct NoopPusher;

#[async_trait::async_trait]
impl FramePusher for NoopPusher {
    async fn push_to_shop(&self, _shop_id: ShopId, _summary: Summary) {}
    async fn push_to_user(&self, _user_id: UserId, _summary: Summary) {}
}

/// The bound for awaiting the loop or one of its effects in a test: long
/// enough for a slow scratch database, short enough that a wedged loop
/// fails the test instead of hanging the suite.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The loop's queue re-probe cadence in the tests (spec §4.4, amended —
/// #446): the reaps the tests assert come from the no-streams rule
/// (immediate once the work is done), so this only sets how often an idle
/// loop with streams attached re-probes the queue — long enough that a
/// healthy loop never re-probes mid-message.
const IDLE_TIMEOUT: Duration = Duration::from_millis(300);

/// The in-memory suite's session identity: the fixture's one user and its
/// Strava id (the same pair the seam suite and the `tb_strava` tests use).
fn test_identity() -> (UserId, StravaId) {
    (UserId::from(1), StravaId::from(42))
}

/// Postgres's "there is already a transaction in progress" notices
/// captured by the suite's logger (sqlx surfaces server notices through
/// the `log` crate, target `sqlx::postgres::notice`): the signal of the
/// cancelled-begin pool-poisoning bug, asserted on by
/// `request_burst_does_not_fire_transaction_notice`.
static NOTICES: AtomicUsize = AtomicUsize::new(0);

struct NoticeCapture;

impl log::Log for NoticeCapture {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        if record.target() == "sqlx::postgres::notice"
            && record.args().to_string().contains("already a transaction")
        {
            NOTICES.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn flush(&self) {}
}

/// Install the notice-capture logger (process-global: the first call wins,
/// the rest are no-ops). Called from `ready()`, so it is installed before
/// any test in this binary asserts on the counter. It captures nothing but
/// the notice, so the other tests' logging behavior is unchanged.
fn install_notice_capture() {
    let _ = log::set_logger(&NoticeCapture);
    log::set_max_level(log::LevelFilter::Warn);
}

/// The suite's `Session`: the loop's session is the user's identity; the
/// Strava API is stubbed by the `TokenCache` seed in `ready()` and the
/// `CANNED_STRAVA` global. These tests never import from Strava (no activity
/// event is processed; the only queued event is a `Stop`, whose handling never
/// touches the API), so the API is canned rather than called.
#[derive(Clone, Copy)]
struct FakeStrava {
    user: UserId,
    shop: Option<ShopId>,
    admin: bool,
}

impl FakeStrava {
    fn new() -> Self {
        let (user, _strava) = test_identity();
        Self {
            user,
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
    fn is_admin(&self) -> bool {
        self.admin
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
    install_notice_capture();
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
    // The loop's Strava path (gear hydration, sync, deauthorize) reads its
    // token from the `TokenCache` (spec #602): seed it with a warm token so
    // the seam never forces a real refresh (which needs Strava credentials
    // the CI runner doesn't have, and would poison the process-wide client).
    let (user, strava) = test_identity();
    seed_token_cache(user, strava, "test-access-token");
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
    let (tx, rx) = request_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    let (request, reply) = Request::new_write(ApiWrite::PartCreate {
        name: "Seam Chain".to_string(),
        vendor: "Shimano".to_string(),
        model: "CN-HG62".to_string(),
        what: CHAIN,
        purchase: datetime!(2024-01-01 00:00 UTC),
    });
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
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
    let part = sole_live(&summary.parts);
    assert_eq!(part.name, "Seam Chain");
    assert_eq!(part.owner, test_identity().0);

    // One frame carrying the same summary reaches the stream.
    let frame = tokio::time::timeout(TEST_TIMEOUT, frame_rx.recv())
        .await
        .expect("the frame must arrive in time")
        .expect("the stream must stay open");
    assert_eq!(frame, summary);
    // No streams left attached: the no-streams rule is the only reap rule
    // (spec §4.4, amended — #446), so the loop's reap needs the receiver
    // gone.
    drop(frame_rx);

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
    let (_tx, rx) = request_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    // No streams attached: the frame receiver must not outlive the loop, or
    // the no-streams reap rule (spec §4.4, amended — #446) never fires.
    drop(_frame_rx);
    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        run(
            DetachedBegin::new(seam.pool.clone()),
            Box::new(FakeStrava::new()),
            rx,
            frames,
            Box::new(NoopPusher),
            events,
            IDLE_TIMEOUT,
            Arc::new(Notify::new()),
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
    let (_tx, rx) = request_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    // No streams attached: the frame receiver must not outlive the loop, or
    // the no-streams reap rule (spec §4.4, amended — #446) never fires.
    drop(_frame_rx);
    let started = std::time::Instant::now();
    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        run(
            DetachedBegin::new(seam.pool.clone()),
            Box::new(FakeStrava::new()),
            rx,
            frames,
            Box::new(NoopPusher),
            events,
            IDLE_TIMEOUT,
            Arc::new(Notify::new()),
        ),
    )
    .await
    .expect("an idle loop must reap in time");
    outcome.expect("a reaped loop returns Ok");
    // The reap came from the no-streams rule (the first idle cycle, spec
    // §4.4, amended — #446) — certainly far under the test bound.
    assert!(
        started.elapsed() < TEST_TIMEOUT,
        "the reap must not take the whole test bound"
    );
}

/// A read as a message end to end on the live adapter (spec #558 §3.6/§8):
/// the `Request::Read` travels the in-memory channel, runs in one live
/// loop-owned transaction (read-only within it by spec), resolves the
/// route's oneshot with the `ReadOutcome`, and pushes **no** stream frame
/// (a read produces no `Summary`). The idle loop then reaps and `run`
/// returns `Ok`.
#[tokio::test]
#[ignore]
async fn read_as_message_round_trip() {
    let seam = ready().await;
    let (tx, rx) = request_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    let (request, reply) = Request::new_read(ApiRead::User);
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
    ));

    // The route's oneshot resolves with the read's data — the unified user
    // read (spec #558 §3.5): the fixture's full state.
    let outcome = tokio::time::timeout(TEST_TIMEOUT, reply)
        .await
        .expect("the route's reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it")
        .expect("the user read must succeed");
    let ReadOutcome::User(state) = outcome else {
        panic!("the User read returns the UserState composite");
    };
    assert_eq!(state.user.id, test_identity().0);
    assert_eq!(state.summary.parts.len(), 17, "the fixture's full state");

    // A read pushes no frame (spec #558 §3.6): the reply resolved after the
    // loop's committed read, so any frame would already be in the channel.
    assert!(
        frame_rx.try_recv().is_err(),
        "a read must push no stream frame"
    );

    // No streams left attached: the no-streams rule is the only reap rule
    // (spec §4.4, amended — #446), so the loop's reap needs the receiver
    // gone.
    drop(frame_rx);
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");
}

/// FIFO total order per user (spec #558 §3.2/§8): a read enqueued after a
/// write sees that write — the loop consumes the channel in order, so the
/// write's transaction commits before the read's `begin`, and the read's
/// `ReadOutcome` carries the written row.
#[tokio::test]
#[ignore]
async fn read_sees_every_earlier_write() {
    let seam = ready().await;
    let (tx, rx) = request_channel();
    let (frames, frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    // The write, then the read — enqueued in order before the loop starts
    // (the channel is the FIFO the loop consumes in order).
    let (write_request, write_reply) = Request::new_write(ApiWrite::PartCreate {
        name: "Seam Chain".to_string(),
        vendor: "Shimano".to_string(),
        model: "CN-HG62".to_string(),
        what: CHAIN,
        purchase: datetime!(2024-01-01 00:00 UTC),
    });
    tx.send(write_request).expect("the channel is open");
    let (read_request, read_reply) = Request::new_read(ApiRead::User);
    tx.send(read_request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
    ));

    // The write commits first (its oneshot resolves with its `Summary`).
    let write_outcome = tokio::time::timeout(TEST_TIMEOUT, write_reply)
        .await
        .expect("the write's reply must resolve in time")
        .expect("the loop must resolve the write's oneshot")
        .expect("the part write must succeed");
    let written = sole_live(&write_outcome.summary().parts);

    // The read — enqueued after the write — sees the committed row.
    let read_outcome = tokio::time::timeout(TEST_TIMEOUT, read_reply)
        .await
        .expect("the read's reply must resolve in time")
        .expect("the loop must resolve the read's oneshot")
        .expect("the user read must succeed");
    let ReadOutcome::User(state) = read_outcome else {
        panic!("the User read returns the UserState composite");
    };
    // The fixture's 17 parts plus the write's — and the written part is in
    // the read's state, exactly as committed.
    assert_eq!(
        state.summary.parts.len(),
        18,
        "the read must see the earlier write"
    );
    assert_eq!(
        state.summary.parts.get(&written.id),
        Some(&written),
        "the read's state must carry the write's row"
    );

    drop(frame_rx);
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");
}

/// The hydration write end to end on the live adapter (spec #558 §4/§8):
/// the `Request::Gear` travels the in-memory channel, runs `update_gear`
/// in one live loop-owned transaction with the loop's session, commits, and
/// pushes exactly one frame — the **union of the parts it created** (one
/// entry per part, published on commit before the oneshot reply). The `User`
/// read enqueued after it (FIFO total order per user) sees the committed
/// hydration: its snapshot carries the new part.
#[tokio::test]
#[ignore]
async fn gear_hydration_frame_and_fifo_read() {
    let seam = ready().await;
    // The offline seam: the hydration's canned Strava JSON (one `b-seam` bike
    // the fixture does not know) in the process-wide `CANNED_STRAVA` global —
    // the loop runs in a spawned task, so the thread-local map is the wrong
    // seam here.
    canned_strava("/athlete", r#"{"bikes":[{"id":"b-seam"}],"shoes":[]}"#);
    canned_strava(
        "/gear/b-seam",
        r#"{"id":"b-seam","name":"Seam Bike","brand_name":"SeamBrand","model_name":"SB1","frame_type":1}"#,
    );
    let (tx, rx) = request_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    // The hydration, then the snapshot read — enqueued in order before the
    // loop starts (the channel is the FIFO the loop consumes in order).
    let (gear_request, gear_reply) = Request::new_gear();
    tx.send(gear_request).expect("the channel is open");
    let (read_request, read_reply) = Request::new_read(ApiRead::User);
    tx.send(read_request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
    ));

    // The hydration commits: the oneshot resolves with the created parts'
    // summary — one entry per created part.
    let summary = tokio::time::timeout(TEST_TIMEOUT, gear_reply)
        .await
        .expect("the gear reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it")
        .expect("the hydration must succeed");
    assert_eq!(
        summary.parts.len(),
        1,
        "the frame is the created parts' union"
    );
    let created = sole_live(&summary.parts);
    assert_eq!(created.name, "Seam Bike");
    assert_eq!(created.source.as_deref(), Some("b-seam"));

    // The created part pushes exactly one frame, published on commit — the
    // reply already resolved, so the frame is already in the channel.
    let frame = tokio::time::timeout(TEST_TIMEOUT, frame_rx.recv())
        .await
        .expect("the frame must arrive in time")
        .expect("the stream must stay open");
    assert_eq!(frame, summary, "the frame is the created parts' union");

    // The read — enqueued after the hydration — sees the committed part:
    // the fixture's 17 parts plus the hydration's, and the snapshot carries
    // the new part exactly as committed.
    let read_outcome = tokio::time::timeout(TEST_TIMEOUT, read_reply)
        .await
        .expect("the read's reply must resolve in time")
        .expect("the loop must resolve the read's oneshot")
        .expect("the user read must succeed");
    let ReadOutcome::User(state) = read_outcome else {
        panic!("the User read returns the UserState composite");
    };
    assert_eq!(
        state.summary.parts.len(),
        18,
        "the FIFO read sees the committed hydration"
    );
    assert_eq!(
        state.summary.parts.get(&created.id),
        Some(&created),
        "the snapshot body carries the new part"
    );

    drop(frame_rx);
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");
}

/// The hydration's common case end to end on the live adapter (spec #558
/// §4/§8): the Strava athlete carries no new gear — `update_gear` creates
/// nothing, the `Summary` is empty, and **no frame is pushed** (exactly
/// today's behavior). The `User` read enqueued after it (FIFO) answers with
/// the fixture's state, unchanged.
#[tokio::test]
#[ignore]
async fn gear_hydration_without_new_gear_pushes_no_frame() {
    let seam = ready().await;
    let fixture_parts = MemStore::prepopulated().snapshot().parts.len();
    // The offline seam: the hydration's canned (empty) Strava athlete in the
    // process-wide `CANNED_STRAVA` global (the loop runs in a spawned task).
    canned_strava("/athlete", r#"{"bikes":[],"shoes":[]}"#);
    let (tx, rx) = request_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    let (gear_request, gear_reply) = Request::new_gear();
    tx.send(gear_request).expect("the channel is open");
    let (read_request, read_reply) = Request::new_read(ApiRead::User);
    tx.send(read_request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
    ));

    // The hydration commits with an empty summary (nothing new).
    let summary = tokio::time::timeout(TEST_TIMEOUT, gear_reply)
        .await
        .expect("the gear reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it")
        .expect("the hydration must succeed");
    assert!(summary.is_empty(), "no new gear: the summary is empty");

    // An empty summary pushes no frame (the gear's commit resolved before
    // this point, so any frame would already be in the channel).
    assert!(
        frame_rx.try_recv().is_err(),
        "no new gear: an empty summary pushes no frame"
    );

    // The FIFO read answers with the fixture's state, unchanged.
    let read_outcome = tokio::time::timeout(TEST_TIMEOUT, read_reply)
        .await
        .expect("the read's reply must resolve in time")
        .expect("the loop must resolve the read's oneshot")
        .expect("the user read must succeed");
    let ReadOutcome::User(state) = read_outcome else {
        panic!("the User read returns the UserState composite");
    };
    assert_eq!(
        state.summary.parts.len(),
        fixture_parts,
        "no new gear: the snapshot is the fixture's state"
    );

    drop(frame_rx);
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");
}

/// A failed write: the domain op errors, the transaction rolls back (no
/// partial row), the oneshot carries the domain error to the route, and
/// the loop keeps going until it reaps.
#[tokio::test]
#[ignore]
async fn failed_write_rolls_back_and_reclaims() {
    let seam = ready().await;
    let fixture_parts = MemStore::prepopulated().snapshot().parts.len();

    let (tx, rx) = request_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    // No streams attached: the frame receiver must not outlive the loop, or
    // the no-streams reap rule (spec §4.4, amended — #446) never fires.
    drop(_frame_rx);
    // Deleting a part that does not exist: `NotFound` from the domain.
    let (request, reply) = Request::new_write(ApiWrite::PartDelete {
        id: PartId::from(999_999),
    });
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
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

/// The onboarding cutover write end to end on the live adapter (spec #558
/// §5.1): the `UserOnboardingSync` message's one loop-owned transaction
/// does the domain half (the status update) **and** the Strava-side
/// `insert_sync` — one message = one loop-owned transaction containing the
/// message's entire work (domain + Strava).
#[tokio::test]
#[ignore]
async fn onboarding_sync_write_queues_the_sync_event() {
    let seam = ready().await;
    let (tx, rx) = request_channel();
    let (frames, _frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    // No streams attached: the frame receiver must not outlive the loop, or
    // the no-streams reap rule (spec §4.4, amended — #446) never fires.
    drop(_frame_rx);
    let (request, reply) = Request::new_write(ApiWrite::UserOnboardingSync { time: 100 });
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
    ));

    // The write commits: the oneshot resolves with the write's (empty)
    // summary — a `User` is in no `Summary`.
    let outcome = tokio::time::timeout(TEST_TIMEOUT, reply)
        .await
        .expect("the route's reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it")
        .expect("the onboarding sync write must succeed");
    assert!(
        outcome.summary().is_empty(),
        "a `User` is in no `Summary`: the write's frame is empty"
    );

    // The Strava-side event is in the queue: the write's one loop-owned
    // transaction did the domain half and the `insert_sync` (spec #558
    // §5.1) — the queue row is committed with the domain half.
    let (user, strava) = test_identity();
    let n = queue_sync_count(&seam.pool, strava, 100).await;
    assert_eq!(n, 1, "the write's transaction must queue the sync event");

    // The loop's next cycle probes the queue and finds the sync: the seam's
    // session never imports from the Strava API, so the task panics in the
    // processing — the expected end of this test's loop.
    let _ = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the loop must settle in time");

    // The event the write's transaction committed is still queued: the
    // panicked processing never committed its own deletion.
    let n = queue_sync_count(&seam.pool, strava, 100).await;
    assert_eq!(n, 1, "the queued sync must survive the panicked processing");
    let _ = user;
}

/// The queue's sync count for `strava` at `event_time` on a fresh
/// connection (the onboarding test's committed-row check).
async fn queue_sync_count(pool: &DbPool, strava: StravaId, event_time: i64) -> i64 {
    let mut conn = pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM strava_events \
         WHERE object_type = 'sync' AND owner_id = $1 AND event_time = $2",
    )
    .bind(i32::from(strava))
    .bind(event_time)
    .fetch_one(&mut **conn)
    .await
    .expect("reading the queue");
    drop(conn);
    n
}

/// The per-user admin sync direct op end to end on the live adapter (spec
/// #558 §5.2/§8): the enqueue transaction — the target's Strava id lookup
/// **and** the `Sync` event, committed together — then the wake, then the
/// await on the consumed-event broadcast (unbounded). The awaited work
/// runs in the target user's loop transaction; the offline seam's canned
/// empty-activities response completes the sync without the Strava API.
#[tokio::test]
#[ignore]
async fn sync_enqueues_wakes_and_awaits() {
    let seam = ready().await;
    let (user, strava) = test_identity();
    // The user's Strava row (the enqueue txn's lookup).
    {
        let mut conn = seam.pool.begin().await.expect("a transaction");
        sqlx::query(
            "INSERT INTO strava_users (id, tendabike_id, refresh_token) VALUES ($1, $2, 'tok')",
        )
        .bind(i32::from(strava))
        .bind(i32::from(user))
        .execute(&mut **conn)
        .await
        .expect("seeding the strava user");
        conn.commit().await.expect("committing the seed");
    }
    // The offline seam: the sync's activities fetch (the `after` cursor is
    // the event time) returns nothing, so the loop's processing deletes
    // the event (consumed) and commits.
    canned_strava("/activities?after=100&per_page=25", "[]");
    // The long idle window: with no stream attached the spawned executor's
    // first probe drains the queue — the wake is the signal that keeps the
    // mechanism intact for a running, idle-waiting executor.
    let registry = Registry::new(
        DetachedBegin::new(seam.pool.clone()),
        Duration::from_secs(3600),
    );

    let outcome = tokio::time::timeout(TEST_TIMEOUT, registry.sync(user, 100))
        .await
        .expect("the admin sync must complete in time");
    outcome.expect("the sync must succeed");

    // The awaited work ran in the target user's loop transaction: the
    // consumed event is gone from the queue for good.
    let n = queue_sync_count(&seam.pool, strava, 100).await;
    assert_eq!(n, 0, "the consumed sync must be gone from the queue");
}

/// The all-users admin sync direct op (spec #558 §5.2/§8): one transaction
/// (`sync_users`) queuing a `Sync` event per (non-disabled) user, then a
/// wake **per queued user**. The test's two executors are running and
/// idle-waiting with streams attached (the long idle window never
/// re-probes mid-test), so the per-user wake is the only drain trigger:
/// without it the queues would stay full for the whole idle window.
#[tokio::test]
#[ignore]
async fn sync_all_wakes_each_running_executor() {
    let seam = ready().await;
    let (user, strava) = test_identity();
    // A second user (the users row plus the Strava row).
    let (user2, strava2) = (UserId::from(2), StravaId::from(43));
    {
        let mut conn = seam.pool.begin().await.expect("a transaction");
        sqlx::query(
            "INSERT INTO users (id, name, firstname, is_admin) VALUES (2, 'Seam Two', 'Seam', false)",
        )
        .execute(&mut **conn)
        .await
        .expect("seeding the second user");
        sqlx::query(
            "INSERT INTO strava_users (id, tendabike_id, refresh_token) VALUES ($1, $2, 'tok')",
        )
        .bind(i32::from(strava))
        .bind(i32::from(user))
        .execute(&mut **conn)
        .await
        .expect("seeding the strava user");
        sqlx::query(
            "INSERT INTO strava_users (id, tendabike_id, refresh_token) VALUES ($1, $2, 'tok')",
        )
        .bind(i32::from(strava2))
        .bind(i32::from(user2))
        .execute(&mut **conn)
        .await
        .expect("seeding the second strava user");
        conn.commit().await.expect("committing the seed");
    }
    // User 2 also goes through the Strava path (its sync's activities
    // fetch): seed its token too (the `ready()` seed covers only user 1).
    seed_token_cache(user2, strava2, "test-access-token");
    // The offline seam: both users' syncs find no activities and complete
    // (the `after` cursor is the event time).
    canned_strava("/activities?after=200&per_page=25", "[]");
    let registry = Registry::new(
        DetachedBegin::new(seam.pool.clone()),
        Duration::from_secs(3600),
    );
    // Both executors running and idle-waiting (streams attached keep them
    // alive; the one-hour re-probe never fires mid-test).
    let (_frames1, _status1) = registry
        .subscribe(Box::new(FakeStrava::new()))
        .await
        .expect("subscribing user 1");
    let (_frames2, _status2) = registry
        .subscribe(Box::new(FakeStrava {
            user: user2,
            ..FakeStrava::new()
        }))
        .await
        .expect("subscribing user 2");
    // Let both executors finish their first (empty) probe and park in the
    // idle sleep: from here on, the per-user wake is the only drain
    // trigger (the re-probe is an hour out).
    tokio::time::sleep(Duration::from_secs(1)).await;

    let outcome = tokio::time::timeout(TEST_TIMEOUT, registry.sync_all(None, 200, false))
        .await
        .expect("the all-users sync must return in time");
    outcome.expect("the all-users sync must succeed");

    // Both queues drain — the per-user wake is the only trigger (the idle
    // re-probe is an hour out).
    let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
    loop {
        let n1 = queue_sync_count(&seam.pool, strava, 200).await;
        let n2 = queue_sync_count(&seam.pool, strava2, 200).await;
        if n1 == 0 && n2 == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "both queued syncs must be consumed (user 1: {n1}, user 2: {n2})"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// ---------------------------------------------------------------------------
// The direct ops (spec #558 §5.2/§8)
// ---------------------------------------------------------------------------

/// Seed the fixture user's Strava row (and, when `queue_event`, one queued
/// event for it) — the direct ops' setup shape.
async fn seed_strava_user(pool: &DbPool, queue_event: bool) {
    let (user, strava) = test_identity();
    let mut conn = pool.begin().await.expect("a transaction");
    sqlx::query(
        "INSERT INTO strava_users (id, tendabike_id, refresh_token) VALUES ($1, $2, 'tok')",
    )
    .bind(i32::from(strava))
    .bind(i32::from(user))
    .execute(&mut **conn)
    .await
    .expect("seeding the strava user");
    if queue_event {
        sqlx::query(
            "INSERT INTO strava_events \
             (object_type, object_id, aspect_type, updates, owner_id, subscription_id, event_time) \
             VALUES ('activity', 7, 'create', '{}', $1, 0, 100)",
        )
        .bind(i32::from(strava))
        .execute(&mut **conn)
        .await
        .expect("seeding the queued event");
    }
    conn.commit().await.expect("committing the seed");
}

/// One test's registry over the live pool (the direct ops' shape): the
/// suite's short idle cadence; the ops under test never rely on the
/// re-probe.
fn registry(pool: &DbPool) -> Registry<DetachedBegin<DbPool>> {
    Registry::new(DetachedBegin::new(pool.clone()), IDLE_TIMEOUT)
}

/// The queue's row count for `strava` on a fresh connection.
async fn queue_count(pool: &DbPool, strava: StravaId) -> i64 {
    let mut conn = pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM strava_events WHERE owner_id = $1")
        .bind(i32::from(strava))
        .fetch_one(&mut **conn)
        .await
        .expect("reading the queue");
    drop(conn);
    n
}

/// The admin rescan direct op (spec #558 §5.2/§8): one transaction
/// (`Activity::rescan_all`), then `stop_all` — every live executor aborted
/// (the watchdog records `Dead` on the abort `JoinError`), its frame
/// channel closed, so the open stream ends (the client's native reconnect
/// + catch-up snapshot re-hydrates it).
#[tokio::test]
#[ignore]
async fn rescan_stops_the_executors() {
    let seam = ready().await;
    let (_user, _strava) = test_identity();
    seed_strava_user(&seam.pool, false).await;
    let reg = registry(&seam.pool);
    let (mut frames, status) = reg
        .subscribe(Box::new(FakeStrava::new()))
        .await
        .expect("subscribing");
    // Let the executor finish its first (empty) probe: it is live when the
    // rescan stops it.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let outcome = tokio::time::timeout(TEST_TIMEOUT, reg.rescan())
        .await
        .expect("the rescan must finish in time");
    outcome.expect("the rescan must succeed");

    // The executor is dead (the abort's watchdog recorded it on the fate
    // channel — the registry's own map is cleared by `stop_all`).
    let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
    loop {
        if *status.borrow() == ExecutorStatus::Dead {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the aborted executor must record Dead"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // The frame channel is closed: the open stream ends (the recv's own
    // `Err(Closed)` — all senders dropped by the abort + map clear).
    let recv = tokio::time::timeout(TEST_TIMEOUT, frames.recv())
        .await
        .expect("the close must be observed in time");
    assert!(recv.is_err(), "the rescan must close the frame channel");
}

/// The webhook accept direct op (spec #558 §5.2/§8): one transaction — the
/// queue insert (or the athlete-deauth branch) — then the wake if a user
/// was queued (fire-and-forget: it never spawns). A failed ingest (an
/// unknown owner) 500s to Strava (its own retry policy applies).
#[tokio::test]
#[ignore]
async fn accept_event_queues_the_event() {
    let seam = ready().await;
    let (_user, strava) = test_identity();
    seed_strava_user(&seam.pool, false).await;
    let reg = registry(&seam.pool);

    // A valid activity event (the webhook's wire shape, deserialized the
    // way the route does).
    let event: InEvent = serde_json::from_value(serde_json::json!({
        "object_type": "activity",
        "object_id": 7,
        "aspect_type": "create",
        "updates": {},
        "owner_id": i32::from(strava),
        "subscription_id": 0,
        "event_time": 100
    }))
    .expect("a valid event");
    let outcome = tokio::time::timeout(TEST_TIMEOUT, reg.accept_event(event))
        .await
        .expect("the accept must finish in time");
    outcome.expect("the accept must succeed");

    // The event is queued (the DB queue is the source of truth; no
    // executor was running, so nothing consumed it).
    let n = queue_count(&seam.pool, strava).await;
    assert_eq!(n, 1, "the accepted event must be queued");

    // An unknown owner: the ingest fails (the 500 to Strava).
    let bad: InEvent = serde_json::from_value(serde_json::json!({
        "object_type": "activity",
        "object_id": 7,
        "aspect_type": "create",
        "updates": {},
        "owner_id": 999,
        "subscription_id": 0,
        "event_time": 100
    }))
    .expect("a valid event");
    let err = reg.accept_event(bad).await.expect_err("unknown owner");
    assert!(
        matches!(err, tb_domain::Error::BadRequest(_)),
        "the unknown-owner ingest must fail, got {err:?}"
    );
}

/// The webhook accept's athlete-deauth branch (spec #558 §5.2): one
/// transaction — the deauthorization disables the user (the queued events
/// dropped, the refresh token cleared) — and no wake (nothing was queued).
#[tokio::test]
#[ignore]
async fn accept_event_athlete_deauth_disables_the_user() {
    let seam = ready().await;
    let (_user, strava) = test_identity();
    seed_strava_user(&seam.pool, true).await;
    let reg = registry(&seam.pool);

    let event: InEvent = serde_json::from_value(serde_json::json!({
        "object_type": "athlete",
        "object_id": 0,
        "aspect_type": "update",
        "updates": {"authorized": "false"},
        "owner_id": i32::from(strava),
        "subscription_id": 0,
        "event_time": 100
    }))
    .expect("a valid event");
    let outcome = tokio::time::timeout(TEST_TIMEOUT, reg.accept_event(event))
        .await
        .expect("the accept must finish in time");
    outcome.expect("the accept must succeed");

    // The user is disabled (the refresh token cleared) and the queue is
    // empty: the deauthorization's one transaction did both.
    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let token: Option<String> =
        sqlx::query_scalar("SELECT refresh_token FROM strava_users WHERE id = $1")
            .bind(i32::from(strava))
            .fetch_one(&mut **conn)
            .await
            .expect("reading the strava user");
    assert!(token.is_none(), "the deauth must clear the refresh token");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM strava_events")
        .fetch_one(&mut **conn)
        .await
        .expect("reading the queue");
    assert_eq!(n, 0, "the deauth must drop the queued events");
    drop(conn);
}

/// The deauthorization direct op (spec #558 §5.2/§8): one transaction —
/// the target's session built in-txn, then `user_deauthorize` (the Strava
/// token revocation, a failure of which is non-fatal and logged, plus the
/// local disable). No wake, no message; the existing queue race is
/// preserved.
#[tokio::test]
#[ignore]
async fn deauthorize_disables_the_user_and_clears_the_queue() {
    let seam = ready().await;
    // The op's Strava token revocation builds the (lazily-constructed)
    // Strava client: the credentials are placeholders (the revocation
    // fails, and `user_deauthorize` treats that as non-fatal).
    unsafe {
        std::env::set_var("CLIENT_ID", "seam");
        std::env::set_var("CLIENT_SECRET", "seam");
    }
    let (user, strava) = test_identity();
    seed_strava_user(&seam.pool, true).await;
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(TEST_TIMEOUT, reg.deauthorize(user))
        .await
        .expect("the deauthorization must finish in time");
    outcome.expect("the deauthorization must succeed");

    // The one transaction disabled the user: the refresh token is cleared
    // and the queued events are gone.
    let n = queue_count(&seam.pool, strava).await;
    assert_eq!(n, 0, "the deauth must drop the queued events");
    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let token: Option<String> =
        sqlx::query_scalar("SELECT refresh_token FROM strava_users WHERE id = $1")
            .bind(i32::from(strava))
            .fetch_one(&mut **conn)
            .await
            .expect("reading the strava user");
    assert!(token.is_none(), "the deauth must clear the refresh token");
    drop(conn);
}

/// The user delete direct op (spec #558 §5.2/§8): one transaction — the
/// target's session, the deauthorization, the row deletions (the user plus
/// their parts, services, plans, attachments, activities, usages). No wake,
/// no message.
#[tokio::test]
#[ignore]
async fn delete_user_removes_the_rows() {
    let seam = ready().await;
    // As the deauth test: the delete's deauthorization half builds the
    // Strava client (placeholder credentials; the failure is non-fatal).
    unsafe {
        std::env::set_var("CLIENT_ID", "seam");
        std::env::set_var("CLIENT_SECRET", "seam");
    }
    let (user, strava) = test_identity();
    seed_strava_user(&seam.pool, true).await;
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(TEST_TIMEOUT, reg.delete_user(user))
        .await
        .expect("the delete must finish in time");
    outcome.expect("the delete must succeed");

    // The rows are gone: the Strava row, the user row, the user's parts.
    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM strava_users WHERE id = $1")
        .bind(i32::from(strava))
        .fetch_one(&mut **conn)
        .await
        .expect("reading the strava users");
    assert_eq!(n, 0, "the strava user row must be gone");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id = $1")
        .bind(i32::from(user))
        .fetch_one(&mut **conn)
        .await
        .expect("reading the users");
    assert_eq!(n, 0, "the user row must be gone");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM parts WHERE owner = $1")
        .bind(i32::from(user))
        .fetch_one(&mut **conn)
        .await
        .expect("reading the parts");
    assert_eq!(n, 0, "the user's parts must be gone");
    drop(conn);
}

// ---------------------------------------------------------------------------
// The executor lifecycle (spec #558 §4.4/§4.5)
// ---------------------------------------------------------------------------

/// Poll the registry's status map until the user's executor reports `want`
/// (bounded by `TEST_TIMEOUT`): the reap/respawn transitions are these
/// lifecycle tests' observables. The map's entry outlives a clean reap —
/// the watchdog records `Reaped` on the fate watch, it does not remove the
/// entry (only `stop_all` clears the map).
async fn wait_for_status<S: tb_exec::TxnSource + Clone>(
    reg: &Registry<S>,
    user: UserId,
    want: ExecutorStatus,
) {
    let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
    loop {
        if reg.status(user).await == Some(want) {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the executor must reach {want:?} in time"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// An attached stream keeps the executor alive past the idle window
/// (spec #558 §4.4, amended — #446): the idle window is the cadence at
/// which the idle loop re-probes the `strava_events` queue, not a reap
/// trigger. `subscribe` is what the SSE endpoint does (it registers the
/// stream on the frame channel and pins the executor), so the test's
/// subscription's fate watch **is** the original executor's status: well
/// past the window it still says `Running` (a reap-and-respawn churn would
/// leave it `Reaped`), and a frame pushed past the window arrives on the
/// stream (the channel is open and forwarding).
#[tokio::test]
#[ignore]
async fn attached_stream_keeps_the_executor_alive() {
    let seam = ready().await;
    let (user, _strava) = test_identity();
    // The user's Strava row: the spawn reads it to build the session.
    seed_strava_user(&seam.pool, false).await;
    // A short idle cadence (the queue re-probe cadence) so the old
    // reap-on-idle policy would have reclaimed this executor quickly.
    let registry = Registry::new(
        DetachedBegin::new(seam.pool.clone()),
        Duration::from_millis(300),
    );

    // The stream connects: `subscribe` is what the SSE endpoint does — it
    // registers this stream on the executor's frame channel and pins the
    // executor (spawning it if needed).
    let (mut frames, status) = registry
        .subscribe(Box::new(FakeStrava::new()))
        .await
        .expect("subscribing");
    wait_for_status(&registry, user, ExecutorStatus::Running).await;

    // Wait well past the idle window (3× the 300ms cadence): under the old
    // policy this executor would have reaped by now.
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // (a) The original executor is still running — no reap, no respawn:
    // the subscription's fate watch still says `Running`.
    assert_eq!(
        *status.borrow(),
        ExecutorStatus::Running,
        "an attached stream must keep its executor running past the idle window"
    );

    // (b) The stream is still alive: a frame pushed well past the idle
    // window arrives on the subscriber's receiver.
    registry.send_frame(user, Summary::default()).await;
    let frame = tokio::time::timeout(TEST_TIMEOUT, frames.recv())
        .await
        .expect("the stream must stay open past the idle window")
        .expect("the stream must not end while a stream is attached");
    assert_eq!(frame, Summary::default());
}

/// The stream survives the executor's reap (spec #558 §4.5, amended —
/// #446): a stream-less (write-only) spawn reaps as soon as its work is
/// done — no streams attached is the only reap rule — and a stream
/// connecting to the reaped executor respawns it and settles: the stream
/// stays alive and a later frame is forwarded (no further reap: the
/// attached stream keeps the executor alive).
#[tokio::test]
#[ignore]
async fn stream_survives_executor_reap() {
    let seam = ready().await;
    let (user, _strava) = test_identity();
    // The user's Strava row: both spawns read it to build their sessions.
    seed_strava_user(&seam.pool, false).await;
    // A short idle cadence (the queue re-probe cadence) so the write-only
    // spawn reaps quickly.
    let registry = Registry::new(
        DetachedBegin::new(seam.pool.clone()),
        Duration::from_millis(300),
    );

    // A write-only spawn: no stream attached, so the executor reaps as
    // soon as its work is done (the no-streams rule — immediate, not after
    // an idle window).
    registry
        .write(
            Box::new(FakeStrava::new()),
            ApiWrite::UserOnboardingPostpone,
        )
        .await
        .expect("the write must succeed");
    wait_for_status(&registry, user, ExecutorStatus::Reaped).await;

    // The stream connects to the reaped executor: the connect respawns it
    // and settles — the stream stays alive and a later frame is forwarded
    // (no further reap: the attached stream keeps the executor alive).
    let (mut frames, _status) = registry
        .subscribe(Box::new(FakeStrava::new()))
        .await
        .expect("subscribing");
    wait_for_status(&registry, user, ExecutorStatus::Running).await;
    registry.send_frame(user, Summary::default()).await;
    let frame = tokio::time::timeout(TEST_TIMEOUT, frames.recv())
        .await
        .expect("the stream must stay open after the respawn")
        .expect("the stream must not end while a stream is attached");
    assert_eq!(frame, Summary::default());
}

/// A burst of user reads — the `GET /api/user/summary` shape (the frontend
/// calls summary for both entering and leaving a shop, so the reads arrive
/// back-to-back) — must not fire Postgres's "there is already a transaction
/// in progress" notice.
///
/// Each arriving read is a cancellation point of the loop's biased select:
/// a `queue_probe` whose `begin` is in flight (the `BEGIN` sent, the
/// response not yet read) is dropped the moment the read branch wins.
/// sqlx increments the client's transaction depth only *after* reading the
/// `BEGIN` response, and its drop guard is a no-op at depth 0 — a
/// cancelled begin therefore returns the connection to the pool with the
/// server-side transaction still active, and the next `begin` on that
/// connection issues a second, noticed `BEGIN`. This test drives the real
/// loop over the real pool with enter/leave read pairs (sub-millisecond
/// apart, like the browser's two fetches) and asserts the notice never
/// fires: the loop's transaction lifecycle must be safe under the
/// select's cancellations.
#[tokio::test]
#[ignore]
async fn request_burst_does_not_fire_transaction_notice() {
    let seam = ready().await;
    let (tx, rx) = request_channel();
    // Pre-fill the channel before the loop spawns: the loop's first select
    // must find work, or its no-streams idle decision reaps it before the
    // burst starts (the reaped loop closes the channel under the senders).
    let (request, warm_reply) = Request::new_read(ApiRead::User);
    tx.send(request).expect("the channel is open");
    let (frames, frame_rx) = broadcast::channel(8);
    let (events, _events_rx) = broadcast::channel(8);
    // A stream stays attached for the whole burst (the user is on the page:
    // the SSE connect is what keeps the loop alive through the gap between
    // the enter-shop and leave-shop summary calls — without it, the
    // probe's `Empty` + no-streams rule reaps the loop in the gap, before
    // the second read can land in the probe's window).

    let mut loop_task = tokio::spawn(run(
        DetachedBegin::new(seam.pool.clone()),
        Box::new(FakeStrava::new()),
        rx,
        frames,
        Box::new(NoopPusher),
        events,
        IDLE_TIMEOUT,
        Arc::new(Notify::new()),
    ));

    // Warm the pool with a full complement of idle connections: the bug's
    // window is between the probe's `BEGIN` being sent and its response
    // being read, and the probe's `begin` only reaches that window when
    // its acquire is instant (an idle connection). With a cold pool the
    // probe's acquire opens a fresh connection (milliseconds), and the
    // next request lands before the `BEGIN` is even sent — a clean
    // pre-wire cancel. Ten parallel acquires give the probe idle
    // connections for the whole burst.
    {
        let mut warmed = Vec::new();
        for _ in 0..10 {
            warmed.push(
                seam.pool
                    .raw()
                    .acquire()
                    .await
                    .expect("a warm-up connection"),
            );
        }
        drop(warmed);
    }
    // The warm-up read's reply: the loop is alive and the pool is warmed.
    let outcome = tokio::time::timeout(TEST_TIMEOUT, warm_reply)
        .await
        .expect("the warm-up reply must resolve in time")
        .expect("the loop must resolve the oneshot, not drop it");
    outcome.expect("the warm-up read must succeed");

    // The burst shape of `GET /api/user/summary`: one client calls it for
    // entering a shop and again for leaving — the two reads arrive
    // back-to-back (the browser's two fetches, milliseconds apart at the
    // backend). The leave-read lands within the few hundred microseconds
    // after the enter-read's commit — exactly the window in which the
    // loop's next select has the queue probe's `begin` in flight (`BEGIN`
    // sent, response unread) — and cancels it mid-wire (the bug's exact
    // interleaving). Pairs are separated by a short gap so the loop
    // settles (probe + idle sleep) between them, like real page navigation.
    const PAIRS: usize = 200;
    let baseline = NOTICES.load(Ordering::SeqCst);
    let tx2 = tx.clone();
    let sender = tokio::spawn(async move {
        for pair in 0..PAIRS {
            for which in 0..2 {
                let (request, reply) = Request::new_read(ApiRead::User);
                if tx2.send(request).is_err() {
                    return; // channel closed: the loop ended; report via the join
                }
                let outcome = tokio::time::timeout(TEST_TIMEOUT, reply)
                    .await
                    .expect("the route's reply must resolve in time")
                    .expect("the loop must resolve the oneshot, not drop it");
                outcome.expect("the user read must succeed");
                // The second read of the pair goes out just after the
                // first's reply (the browser's back-to-back fetches, with
                // sub-millisecond jitter): the reply is the moment the
                // loop's next select starts the queue probe, and this send
                // lands in the probe begin's in-flight window (after the
                // `BEGIN` is sent, before its response is read).
                if which == 0 {
                    tokio::time::sleep(Duration::from_micros(30 + (pair % 120) as u64)).await;
                }
            }
            // The gap between page navigations: the loop settles (probe +
            // idle sleep) before the next pair.
            tokio::time::sleep(Duration::from_millis(5 + (pair % 15) as u64)).await;
        }
    });
    drop(tx);
    let _ = sender.await; // a closed channel ends the sender quietly; the loop's fate is below
    drop(frame_rx); // the stream disconnects only after the burst: the no-streams rule can now reap

    // The channel is closed and no stream is attached: the loop reaps.
    let outcome = tokio::time::timeout(TEST_TIMEOUT, &mut loop_task)
        .await
        .expect("the idle loop must reap in time")
        .expect("the loop must not panic");
    outcome.expect("a reaped loop returns Ok");

    let notices = NOTICES.load(Ordering::SeqCst) - baseline;
    assert_eq!(
        notices, 0,
        "the burst fired {notices} 'there is already a transaction in progress' \
         notice(s): a cancelled probe begin left a pooled connection inside a \
         server-side transaction the client does not know about"
    );
}
