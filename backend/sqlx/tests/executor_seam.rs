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
use std::time::Duration;

use serde::de::DeserializeOwned;
use tb_domain::test_support::{MemStore, part_type_ids::CHAIN, sole_live};
use tb_domain::{ApiRead, ApiWrite, Error, PartId, ReadOutcome, Session, ShopId, TbResult, UserId};
use tb_exec::{Request, request_channel, run};
use tb_sqlx::DbPool;
use tb_strava::{StravaDataStore, StravaId, StravaSession};
use time::macros::datetime;
use tokio::sync::{MutexGuard, Notify, broadcast};

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
        _store: &mut impl StravaDataStore,
    ) -> TbResult<T> {
        panic!("the executor seam never imports from the Strava API (got {uri})")
    }
    async fn deauthorize(&mut self, _store: &mut impl StravaDataStore) -> TbResult<()> {
        panic!("the executor seam never deauthorizes a user")
    }
}

/// The gear tests' `StravaSession` (spec #558 §4): the loop's session is the
/// user's identity plus the Strava API client, and the client is canned
/// JSON. `athlete` is the canned `/athlete` response: a `b-seam` bike (the
/// hydration creates it) or an empty athlete (no new gear — the common
/// case).
#[derive(Clone, Copy)]
struct GearStrava {
    /// The canned `/athlete` response.
    athlete: &'static str,
}

impl GearStrava {
    /// A Strava athlete with one bike the fixture does not know: the
    /// hydration's `update_gear` creates it.
    fn new_gear() -> Self {
        Self {
            athlete: r#"{"bikes":[{"id":"b-seam"}],"shoes":[]}"#,
        }
    }

    /// A Strava athlete with no gear: the hydration creates nothing.
    fn no_new_gear() -> Self {
        Self {
            athlete: r#"{"bikes":[],"shoes":[]}"#,
        }
    }
}

impl Session for GearStrava {
    fn user_id(&self) -> UserId {
        test_identity().0
    }
    fn shop(&self) -> Option<ShopId> {
        None
    }
    fn set_shop(&mut self, _shop: Option<ShopId>) -> TbResult<()> {
        Ok(())
    }
    fn is_admin(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
impl StravaSession for GearStrava {
    fn strava_id(&self) -> StravaId {
        test_identity().1
    }
    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        _store: &mut impl StravaDataStore,
    ) -> TbResult<T> {
        let json = match uri {
            "/athlete" => self.athlete,
            "/gear/b-seam" => {
                r#"{"id":"b-seam","name":"Seam Bike","brand_name":"SeamBrand","model_name":"SB1","frame_type":1}"#
            }
            _ => panic!("unexpected Strava call: {uri}"),
        };
        serde_json::from_str(json).map_err(|err| Error::AnyFailure(err.into()))
    }
    async fn deauthorize(&mut self, _store: &mut impl StravaDataStore) -> TbResult<()> {
        panic!("the gear tests never deauthorize a user")
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
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
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
            seam.pool.clone(),
            FakeStrava::new(),
            rx,
            frames,
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
            seam.pool.clone(),
            FakeStrava::new(),
            rx,
            frames,
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
    let (request, reply) = Request::new_read(ApiRead::User { shop: None });
    tx.send(request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
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
    let (read_request, read_reply) = Request::new_read(ApiRead::User { shop: None });
    tx.send(read_request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
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
    let (tx, rx) = request_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    // The hydration, then the snapshot read — enqueued in order before the
    // loop starts (the channel is the FIFO the loop consumes in order).
    let (gear_request, gear_reply) = Request::new_gear();
    tx.send(gear_request).expect("the channel is open");
    let (read_request, read_reply) = Request::new_read(ApiRead::User { shop: None });
    tx.send(read_request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        seam.pool.clone(),
        GearStrava::new_gear(),
        rx,
        frames,
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
    let (tx, rx) = request_channel();
    let (frames, mut frame_rx) = broadcast::channel(16);
    let (events, _events_rx) = broadcast::channel(16);
    let (gear_request, gear_reply) = Request::new_gear();
    tx.send(gear_request).expect("the channel is open");
    let (read_request, read_reply) = Request::new_read(ApiRead::User { shop: None });
    tx.send(read_request).expect("the channel is open");

    let mut loop_task = tokio::spawn(run(
        seam.pool.clone(),
        GearStrava::no_new_gear(),
        rx,
        frames,
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
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
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
        seam.pool.clone(),
        FakeStrava::new(),
        rx,
        frames,
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
