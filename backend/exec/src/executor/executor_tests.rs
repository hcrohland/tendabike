/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! Unit tests for the executor loop (ADR-0005, executable spec #446 §10;
//! ADR-0007, executable spec #558 §3/§4). The loop's control flow — message
//! selection, reclaim decision, frame construction — is structured as pure
//! units testable with a channel and dummy futures (no store, no database).
//!
//! The transaction seam is a `mockall` `mock!` witness: [`MockConn`] is a
//! single type accumulating the full `Store` (nine sub-traits + the marker)
//! plus `StravaStore` and `Txn` — the `TxnSource::Conn` bound. The per-request
//! tests (`run_request`) assert on **canned `mock!` outcomes** (the store
//! returns the values the test sets on `expect_*`): they verify the
//! **executor's** behavior — how it drives the store (`begin` → work →
//! `commit`/`rollback`) and what it does with the results (frame routing,
//! the backoff discipline, the oneshot reply) — not the store's internal
//! behavior. The store's behavior (and the domain logic that runs over it)
//! is verified against real Postgres in `tb_sqlx`'s seam suites.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use tb_domain::test_support::{
    fixtures::sample_purchase_date, mock_session, mock_session_with_shop, part_type_ids::CHAIN,
    sole_live,
};
use tb_domain::{
    ApiRead, ApiWrite, Error, Notification, NotificationId, NotificationKind,
    NotificationLifecycle, NotificationState, NotificationSubject, OnboardingStatus, Part, PartId,
    PartTypeId, ReadOutcome, Service, ServiceId, Shop, ShopId, SubscriptionId, Summary, TbResult,
    Usage, UsageId, User, UserId, UserStat, UserState, WriteOutcome,
};
use tb_provider::{Probe, ProviderLane};
use tb_strava::event::Event;
use time::OffsetDateTime;
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;

use super::{
    DbBackoff, LoopContext, Select, next_action, run_request, select_message,
    sleep_or_take_request, stop_remaining,
};
use crate::message::{Request, push_frame};
use crate::{
    Action, DeliveryContext, FramePusher, Message, MockDomainFacade, Txn, TxnSource,
    request_channel,
};

// --- The `mock!` connection witness (issue #631) ---
// --- The test doubles for the executor's seams (issue #631, spec #795) ---
//
// [`MockConn`] is a `mockall` `mock!` witness carrying **only** the [`Txn`]
// lifecycle (commit/rollback) — the `TxnSource::Conn` bound. The domain and
// provider seams are mocked separately (the [`MockDomainFacade`] and the
// [`MockLane`]), so the connection is just the transaction the loop opens,
// works in, and closes.
//
// [`MockLane`] is the `mockall` `mock!` witness for the [`ProviderLane`]
// seam: the executor's door into the provider ops (`probe`, `process`,
// `insert_sync`, `update_gear`, `all_stats`). Its `Store` is [`MockConn`]
// (the `FakeSource`'s connection), so the loop's transaction ties to it.
//
// `#[async_trait]` on each impl block is the trick that makes the `async fn`
// methods desugar correctly in the generated code (see
// `docs/research/mockall-conn-witness.md` §1).
//
// Test-support only: this module is `#[cfg(test)]`, so no production `pub`
// surface is widened. `mockall` is a dev-dependency of `tb_exec` only.

mockall::mock! {
    pub Conn {}

    #[async_trait::async_trait]
    impl Txn for Conn {
        async fn commit(self) -> TbResult<()>;
        async fn rollback(self) -> TbResult<()>;
    }
}

mockall::mock! {
    pub Lane {}

    #[async_trait::async_trait]
    impl ProviderLane for Lane {
        type Event = Event;
        type Store = MockConn;
        async fn probe(&self, user_id: UserId, store: &mut MockConn) -> Probe<Event>;
        async fn process(&self, user_id: UserId, store: &mut MockConn) -> Option<Summary>;
        async fn insert_sync(
            &self,
            user_id: UserId,
            time: i64,
            auto: bool,
            store: &mut MockConn,
        ) -> TbResult<()>;
        async fn update_gear(&self, user_id: UserId, store: &mut MockConn) -> TbResult<Vec<PartId>>;
        async fn all_stats(&self, store: &mut MockConn) -> TbResult<Vec<UserStat>>;
    }
}

// The `mockall::mock!` macro does not derive `Clone` (the `Core` requires
// `P: Clone`). The `Core` tests never drive the lane, so a `Clone` impl
// that returns a fresh mock (no expectations) is safe.
impl Clone for MockLane {
    fn clone(&self) -> Self {
        Self::new()
    }
}

// The `#[automock]` mock does not derive `Clone` (the `Registry` requires
// `D: Clone`). The tests never drive the facade's expectations after the
// clone, so a `Clone` impl that returns a fresh mock is safe.
impl Clone for MockDomainFacade<MockConn> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

/// A `TxnSource` whose `begin` fails (the DB-down corner) or hands out the
/// test's configured [`MockConn`] — the store the executor's per-request
/// dispatch drives against canned outcomes. `begin` is called exactly once
/// per `run_request` (one loop-owned transaction), so the mock is taken out
/// of the `Mutex` on that call.
#[derive(Clone)]
pub(crate) struct FakeSource {
    begin_fails: bool,
    /// The conns `begin` hands out, in order (one per call): most tests
    /// install one (one loop-owned transaction per `run_request`); the
    /// `subscribe_shop` tests install one per ownership read.
    conns: Arc<Mutex<VecDeque<MockConn>>>,
}

impl FakeSource {
    /// A source whose `begin` fails with a `DatabaseFailure` (the DB-down
    /// corner): no store is ever handed out.
    pub(crate) fn failing() -> Self {
        Self {
            begin_fails: true,
            conns: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// A source that hands out `conn` on its (single) `begin`.
    fn with(conn: MockConn) -> Self {
        Self::with_conns(vec![conn])
    }

    /// A source that hands out the given conns in order (one per `begin`).
    fn with_conns(conns: Vec<MockConn>) -> Self {
        Self {
            begin_fails: false,
            conns: Arc::new(Mutex::new(conns.into_iter().collect())),
        }
    }
}

#[async_trait::async_trait]
impl TxnSource for FakeSource {
    type Conn = MockConn;

    async fn begin(&self) -> TbResult<MockConn> {
        if self.begin_fails {
            return Err(Error::DatabaseFailure(anyhow::anyhow!(
                "the fake database is down"
            )));
        }
        self.conns.lock().unwrap().pop_front().ok_or_else(|| {
            Error::DatabaseFailure(anyhow::anyhow!("begin was called more than once"))
        })
    }
}

/// A future that never resolves: stands in for the Strava queue probe being
/// blocked (a slow or busy queue), so the select has a second, non-ready
/// branch without a store.
struct Never;

impl Future for Never {
    type Output = Option<Probe<Event>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

/// A future that immediately resolves with a ready probe outcome.
async fn ready_probe(probe: Probe<Event>) -> Option<Probe<Event>> {
    Some(probe)
}

fn request() -> (Request, oneshot::Receiver<TbResult<WriteOutcome>>) {
    Request::new_write(
        Box::new(mock_session(UserId::from(1))),
        ApiWrite::UserOnboardingPostpone,
    )
}

/// The context the per-request tests drive: the loop's wiring (source,
/// facade, lane, frames, pusher) with the context's starting state (a fresh
/// backoff, the first-idle flag set). The consumed-event channel, the
/// re-probe cadence, and the wake signal are never touched by `run_request`,
/// so they are fresh dummies.
fn test_context(
    source: FakeSource,
    facade: MockDomainFacade<MockConn>,
    lane: MockLane,
    frames: broadcast::Sender<Summary>,
    pusher: Box<dyn FramePusher>,
) -> LoopContext<FakeSource, MockLane, MockDomainFacade<MockConn>> {
    LoopContext::new(
        source,
        facade,
        UserId::from(1),
        frames,
        pusher,
        lane,
        broadcast::channel(64).0,
        Duration::from_secs(300),
        Arc::new(tokio::sync::Notify::new()),
    )
}

// --- `next_action`: the reclaim decision ---

#[test]
fn next_action_prefer_write_over_everything() {
    // A client action must never wait behind a queued event or active
    // streams (spec §4.3: the select is biased).
    assert_eq!(next_action(true, true, 3), Action::ProcessWrite);
}

#[test]
fn next_action_strava_when_no_write() {
    assert_eq!(next_action(false, true, 0), Action::ProcessStrava);
}

#[test]
fn next_action_reap_with_no_streams() {
    // Nothing pending and no one listening: exit — the only reap rule
    // (spec §4.4, amended: the idle window no longer reaps, so no idle
    // arguments).
    assert_eq!(next_action(false, false, 0), Action::Reap);
}

#[test]
fn next_action_sleep_while_streams_listen() {
    // Streams still attached: stay alive — the idle window is the queue
    // re-probe cadence, not a reap trigger (spec §4.4, amended).
    assert_eq!(next_action(false, false, 1), Action::Sleep);
    assert_eq!(next_action(false, false, 2), Action::Sleep);
}

// --- `select_message`: the biased select ---

#[tokio::test]
async fn select_api_write_wins_over_ready_strava() {
    // Both ready at the same instant: the write branch is polled first.
    let (_tx, mut rx) = request_channel();
    let (req, _reply) = request();
    let _ = _tx.send(req);
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(
        rx.recv(),
        ready_probe(Probe::Event(Event::default())),
        &mut wake_fut,
    )
    .await;
    assert!(
        matches!(
            sel,
            Select::Request(ref req)
                if matches!(
                    req,
                    Request::Write { write, .. }
                        if matches!(write, ApiWrite::UserOnboardingPostpone)
                )
        ),
        "the ready API write must win the biased select, got {sel:?}"
    );
}

#[tokio::test]
async fn select_strava_when_no_write_pending() {
    // An empty-but-open channel parks the write branch; the ready Strava
    // branch is then taken.
    let (_tx, mut rx) = request_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(
        rx.recv(),
        ready_probe(Probe::Event(Event::default())),
        &mut wake_fut,
    )
    .await;
    assert!(
        matches!(sel, Select::Strava(ref e) if *e == Event::default()),
        "a ready Strava event is taken when no write is pending, got {sel:?}"
    );
}

#[tokio::test]
async fn select_idle_when_queue_empty_and_no_write() {
    // Neither branch ready-yet-interesting: an empty queue (committed probe)
    // with no pending write is the idle state the reclaim policy decides on.
    let (_tx, mut rx) = request_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Empty), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::Idle),
        "empty queue + no write is Idle, got {sel:?}"
    );
}

#[tokio::test]
async fn select_idle_when_channel_closed_and_queue_empty() {
    // A dropped sender resolves `recv()` with `None`; with an empty queue the
    // select is idle (in production the sender lives for the process, so this
    // is a shutdown corner, not a normal state).
    let (tx, mut rx) = request_channel();
    drop(tx);
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Empty), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::Idle),
        "closed channel + empty queue is Idle, got {sel:?}"
    );
}

#[tokio::test]
async fn select_strava_error_is_its_own_state() {
    // A failed probe is reported, not silently read as "empty": the loop
    // pauses instead of deciding the queue drained.
    let (_tx, mut rx) = request_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Error), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::StravaError),
        "a failed probe is StravaError, got {sel:?}"
    );
}

#[tokio::test]
async fn select_limited_when_stop_active() {
    // An active global rate-limit Stop is its own select state (spec #446
    // §4.6): the loop sleeps until the deadline as a branch of the select,
    // interruptible by API writes.
    let (_tx, mut rx) = request_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Limited(1234)), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::Limited(1234)),
        "an active Stop is Select::Limited, got {sel:?}"
    );
}

// --- `stop_remaining`: the Stop deadline as a sleep duration ---

#[test]
fn stop_remaining_until_the_deadline() {
    assert_eq!(stop_remaining(1000, 400), Duration::from_secs(600));
}

#[test]
fn stop_remaining_expired_is_zero() {
    // An expired (or now-exactly-at) deadline sleeps nothing: the next probe
    // deletes the Stop and reads on.
    assert_eq!(stop_remaining(1000, 1000), Duration::ZERO);
    assert_eq!(stop_remaining(400, 1000), Duration::ZERO);
}

// --- `DbBackoff`: the in-memory DB-failure backoff (spec #446 §4.6) ---

#[test]
fn db_backoff_doubles_until_the_cap() {
    // Exponential, capped at 30s (spec #446 §4.6): a broken database must not
    // hot-spin the loop, and a recovering one is probed at human pace.
    let mut b = DbBackoff::new();
    assert_eq!(b.record_failure(), Duration::from_secs(1));
    assert_eq!(b.record_failure(), Duration::from_secs(2));
    assert_eq!(b.record_failure(), Duration::from_secs(4));
    assert_eq!(b.record_failure(), Duration::from_secs(8));
    assert_eq!(b.record_failure(), Duration::from_secs(16));
    assert_eq!(
        b.record_failure(),
        Duration::from_secs(30),
        "16s x 2 caps at 30s"
    );
    assert_eq!(
        b.record_failure(),
        Duration::from_secs(30),
        "and stays capped"
    );
}

#[test]
fn db_backoff_resets_after_a_success() {
    // A healthy database (any committed transaction) restarts the backoff at
    // its base: the next failure gets the short pause, not the cap.
    let mut b = DbBackoff::new();
    b.record_failure();
    b.record_failure();
    b.reset();
    assert_eq!(
        b.record_failure(),
        Duration::from_secs(1),
        "a success resets the backoff to its base"
    );
}

#[tokio::test]
async fn select_waits_for_the_write_over_a_slow_queue() {
    // The queue probe is pending (slow); a write that arrives while waiting is
    // taken the moment it is ready — the write never waits behind the probe.
    let (tx, mut rx) = request_channel();
    let sel = tokio::spawn(async move {
        let _ = tx.send(request().0);
        let wake = tokio::sync::Notify::new();
        let mut wake_fut = Box::pin(wake.notified());
        select_message(rx.recv(), Never, &mut wake_fut).await
    });
    // Give the task a moment to park on the never-ready queue probe; the send
    // in the same task has already happened, so the write branch is ready.
    let sel = tokio::time::timeout(Duration::from_secs(5), sel)
        .await
        .expect("a queued write resolves the select")
        .expect("the task does not panic");
    assert!(
        matches!(sel, Select::Request(_)),
        "a write arriving over a slow queue wins, got {sel:?}"
    );
}

#[tokio::test]
async fn select_wake_fired_before_the_select_is_wake() {
    // Permit semantics: a notify that fires before the `notified()` future is
    // created is not lost — the select resolves to the wake state (the loop
    // re-probes the queue on its next cycle), even with a slow probe parked.
    let (_tx, mut rx) = request_channel();
    let wake = tokio::sync::Notify::new();
    wake.notify_one();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = tokio::time::timeout(
        Duration::from_secs(5),
        select_message(rx.recv(), Never, &mut wake_fut),
    )
    .await
    .expect("a pre-fired wake resolves the select");
    assert!(
        matches!(sel, Select::Wake),
        "a fired wake is the wake state, got {sel:?}"
    );
}

#[tokio::test]
async fn select_wake_firing_while_parked_is_wake() {
    // The queue probe is pending (slow) and no write is queued; a wake that
    // fires while the select is parked resolves it to the wake state — the
    // loop re-probes the queue on its next cycle.
    let (_tx, mut rx) = request_channel();
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    let wake_in_task = wake.clone();
    let task = tokio::spawn(async move {
        let mut wake_fut = Box::pin(wake_in_task.notified());
        select_message(rx.recv(), Never, &mut wake_fut).await
    });
    // Let the task park on the never-ready probe, then fire the wake.
    tokio::time::sleep(Duration::from_millis(50)).await;
    wake.notify_one();
    let sel = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("a fired wake resolves the select")
        .expect("the task does not panic");
    assert!(
        matches!(sel, Select::Wake),
        "a wake firing mid-park is the wake state, got {sel:?}"
    );
}

// --- `Message` and the frame push ---

#[test]
fn message_carries_both_sources() {
    let write = ApiWrite::UserOnboardingPostpone;
    assert_eq!(Message::ApiWrite(write.clone()), Message::ApiWrite(write));
    assert_eq!(
        Message::Strava(Event::default()),
        Message::Strava(Event::default())
    );
}

#[test]
fn request_bundles_write_and_reply() {
    let (req, mut reply) = request();
    // Open, but nothing sent yet: the reply is pending, not closed.
    assert!(
        matches!(
            reply.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ),
        "the reply must be open"
    );
    let (tx, mut rx) = request_channel();
    tx.send(req)
        .expect("an unbounded channel accepts the request");
    let Request::Write { write, .. } = rx.try_recv().expect("the request round-trips the channel")
    else {
        panic!("the channel carries the write request");
    };
    assert_eq!(write, ApiWrite::UserOnboardingPostpone);
}

#[test]
fn request_bundles_read_and_reply() {
    // The read variant bundles the `ApiRead` descriptor and its oneshot the
    // same way the write does (spec #558 §3.2).
    let (req, mut reply) = Request::new_read(
        Box::new(mock_session(UserId::from(1))),
        ApiRead::PartCategories,
    );
    assert!(
        matches!(
            reply.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ),
        "the reply must be open"
    );
    let (tx, mut rx) = request_channel();
    tx.send(req)
        .expect("an unbounded channel accepts the request");
    let Request::Read { read, .. } = rx.try_recv().expect("the request round-trips the channel")
    else {
        panic!("the channel carries the read request");
    };
    assert_eq!(read, ApiRead::PartCategories);
}

#[test]
fn frame_push_delivers_non_empty_summaries() {
    let (tx, mut rx) = broadcast::channel(4);
    let mut frame = Summary::default();
    let usage = Usage::default();
    frame.usages.upsert(usage);

    push_frame(&tx, &frame);
    let got = rx
        .try_recv()
        .expect("a non-empty summary pushes exactly one frame");
    assert_eq!(got, frame);
}

#[test]
fn frame_push_delivers_tombstone_only_summaries() {
    let (tx, mut rx) = broadcast::channel(4);
    let mut frame = Summary::default();
    frame.usages.tombstone(Usage::default().id);

    // A tombstone-only summary is not empty: the delete must still reach
    // the client (spec §4.5).
    push_frame(&tx, &frame);
    let got = rx
        .try_recv()
        .expect("a tombstone-only summary pushes exactly one frame");
    assert_eq!(got, frame);
}

#[test]
fn frame_push_skips_empty_summaries() {
    let (tx, mut rx) = broadcast::channel(4);
    push_frame(&tx, &Summary::default());
    assert!(
        rx.try_recv().is_err(),
        "an empty summary pushes nothing (spec §4.5)"
    );
}

// --- Delivery context construction (spec #706) ---
//
// The loop hands the pusher a [`DeliveryContext`] (the acting user, the
// session's shop, the resolved part owner) plus the committed `Summary`; the
// routing itself — "where does the frame go?" — is the pusher's (the
// `Core::deliver` tests in `registry/core.rs`). The tests here assert the
// loop constructs the right context, not the routing.

/// A `FramePusher` test double: records the `deliver` calls (the
/// [`DeliveryContext`] and the `Summary`) so the loop's delivery intent is
/// observable. `Clone` shares the recording (the `Arc`), so a test keeps an
/// observation handle after moving the pusher into the context.
#[derive(Default, Clone)]
pub(crate) struct RecordingPusher {
    calls: std::sync::Arc<std::sync::Mutex<Vec<DeliverCall>>>,
}

/// One recorded delivery: the routing inputs the loop handed the pusher and
/// the frame it carried.
#[derive(Debug, Clone, PartialEq)]
struct DeliverCall {
    context: DeliveryContext,
    summary: Summary,
}

impl RecordingPusher {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The recorded deliveries (a clone: the test reads them after the loop).
    fn calls(&self) -> Vec<DeliverCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl FramePusher for RecordingPusher {
    async fn deliver(&self, context: DeliveryContext, summary: Summary) {
        self.calls
            .lock()
            .unwrap()
            .push(DeliverCall { context, summary });
    }
}

/// A part owned by `owner` (the routing's `push_to_user` target).
pub(crate) fn part_owned_by(owner: UserId) -> Part {
    Part {
        id: PartId::from(1),
        owner,
        what: PartTypeId::from(1),
        name: "Test Part".to_string(),
        vendor: "Vendor".to_string(),
        model: "Model".to_string(),
        purchase: OffsetDateTime::UNIX_EPOCH,
        last_used: OffsetDateTime::UNIX_EPOCH,
        disposed_at: None,
        usage: UsageId::default(),
        source: None,
        shop: None,
    }
}

/// The canned Strava-imported bike the gear test's store returns: one part
/// carrying the Strava source `b123`.
fn part_test_bike() -> Part {
    Part {
        id: PartId::from(1),
        owner: UserId::from(1),
        what: PartTypeId::from(1),
        name: "Test Bike".to_string(),
        vendor: "TestBrand".to_string(),
        model: "M1".to_string(),
        purchase: OffsetDateTime::UNIX_EPOCH,
        last_used: OffsetDateTime::UNIX_EPOCH,
        disposed_at: None,
        usage: UsageId::default(),
        source: Some("b123".to_string()),
        shop: None,
    }
}

/// A canned `User` row the read tests' store returns.
fn user_record(id: UserId) -> User {
    User {
        id,
        name: "Bike".to_string(),
        firstname: "Tenda".to_string(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Completed,
    }
}

/// A non-empty `Summary` (a frame the shop/user streams should deliver).
pub(crate) fn non_empty_summary() -> Summary {
    let mut summary = Summary::default();
    summary.usages.upsert(Usage::default());
    summary
}

/// A canned `Shop` owned by `owner` (the `subscribe_shop` ownership read).
pub(crate) fn shop_owned_by(owner: UserId) -> Shop {
    Shop {
        id: ShopId::from(7),
        owner,
        name: "Test Shop".to_string(),
        description: None,
        auto_approve: true,
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// A `Summary` carrying a notification for `recipient`.
pub(crate) fn summary_with_notification(recipient: UserId) -> Summary {
    let notification = Notification {
        id: NotificationId::from(Uuid::now_v7()),
        recipient,
        lifecycle: NotificationLifecycle::Persistent,
        kind: NotificationKind::SubRequest,
        subject: NotificationSubject::Subscription(SubscriptionId::from(1)),
        message: None,
        state: NotificationState::New,
        created_at: sample_purchase_date(),
    };
    let mut summary = Summary::default();
    summary.notifications.upsert(notification);
    summary
}

/// A non-empty `Summary` with a single part owned by `owner`: the frame a
/// shop-context write would deliver.
pub(crate) fn summary_owned_by(owner: UserId) -> Summary {
    let mut summary = Summary::default();
    summary.parts.upsert(part_owned_by(owner));
    summary
}

/// A non-empty `Summary` with a service referencing a part (no `Part` in the
/// summary — the common shape for service operations): the frame a
/// shop-context service write would deliver. The part owner is not derivable
/// from this summary; the executor resolves it via the store.
pub(crate) fn summary_with_service() -> Summary {
    let service = Service {
        id: ServiceId::default(),
        part_id: PartId::from(1),
        time: OffsetDateTime::UNIX_EPOCH,
        redone: OffsetDateTime::UNIX_EPOCH,
        name: "Test Service".to_string(),
        notes: String::new(),
        usage: UsageId::default(),
        successor: None,
        plans: Vec::new(),
    };
    let mut summary = Summary::default();
    summary.services.upsert(service);
    summary
}

/// A transient `PartRegistered` notification for `recipient` (the frame a part
/// registration produces): the shared literal the cross-recipient and
/// recipient-fan-out routing tests build their summaries from. It returns the
/// `Notification` value — not a `Summary` — because those tests add it to a
/// larger summary (parts + shops) of their own.
pub(crate) fn notification_part_registered(recipient: UserId) -> Notification {
    Notification {
        id: NotificationId::from(Uuid::now_v7()),
        recipient,
        lifecycle: NotificationLifecycle::Transient,
        kind: NotificationKind::PartRegistered,
        subject: NotificationSubject::Part(PartId::from(1)),
        message: None,
        state: NotificationState::New,
        created_at: sample_purchase_date(),
    }
}

// --- `select_message` and the sleep: the read's select discipline ---

#[tokio::test]
async fn select_read_wins_over_ready_strava() {
    // Both ready at the same instant: the request branch (a read) is polled
    // first — a read is never delayed by the Strava side (spec #558 §3.7).
    let (tx, mut rx) = request_channel();
    let (req, _reply) = Request::new_read(
        Box::new(mock_session(UserId::from(1))),
        ApiRead::PartCategories,
    );
    let _ = tx.send(req);
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(
        rx.recv(),
        ready_probe(Probe::Event(Event::default())),
        &mut wake_fut,
    )
    .await;
    assert!(
        matches!(sel, Select::Request(ref req) if matches!(req, Request::Read { .. })),
        "the ready read must win the biased select, got {sel:?}"
    );
}

#[tokio::test]
async fn read_arriving_during_a_sleep_is_taken_immediately() {
    // A read enqueued while the loop is in a Stop/DbBackoff sleep is taken
    // the moment it is ready (spec #558 §3.7: a read is never delayed by
    // the Strava `Stop` nor by the `DbBackoff`), not at the sleep's expiry.
    let (tx, mut rx) = request_channel();
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    let wake_in_task = wake.clone();
    let task = tokio::spawn(async move {
        let mut wake_fut = Box::pin(wake_in_task.notified());
        sleep_or_take_request(&mut rx, Duration::from_secs(30), &mut wake_fut).await
    });
    // Let the task park in the 30s sleep, then enqueue the read.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (req, _reply) = Request::new_read(
        Box::new(mock_session(UserId::from(1))),
        ApiRead::PartCategories,
    );
    tx.send(req).expect("the channel is open");
    let got = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("the read must interrupt the sleep, not wait its 30s")
        .expect("the task does not panic");
    assert!(
        matches!(got, Some(Request::Read { .. })),
        "the mid-sleep read is taken immediately, got {got:?}"
    );
}

// --- The per-request dispatch against canned store outcomes (spec #558 §3/§4) ---
//
// Each test sets canned `expect_*` returns on the [`MockConn`] (the store the
// executor drives), runs the request through `run_request`, and asserts the
// executor's handling: the transaction lifecycle (`begin` → work →
// `commit`/`rollback`), the oneshot reply, the frame routing, and the backoff
// discipline. The canned store outcomes are independent of any real store —
// the executor's behavior is what is under test.

// --- The read's backoff discipline (spec #558 §3.7) ---

#[tokio::test]
async fn failed_read_never_arms_the_backoff_and_the_loop_survives() {
    // A failed read (a failed `begin`) resolves the oneshot with the error
    // and **never arms the backoff** (spec #558 §3.7: the queue probe is
    // the DB health monitor, and read traffic must not push the 30s cap
    // up) — and the dispatch returns `Ok`, so the loop survives. The store is
    // never reached: `begin` fails fast.
    let source = FakeSource::failing();
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_read(
        Box::new(mock_session(UserId::from(1))),
        ApiRead::PartCategories,
    );
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(
        source,
        MockDomainFacade::new(),
        MockLane::new(),
        frames,
        Box::new(pusher),
    );
    ctx.backoff.record_failure();
    ctx.backoff.record_failure(); // prime: the next pause would be 4s
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a failed read must not end the loop");
    let err = reply
        .await
        .expect("the oneshot resolves")
        .expect_err("a failed begin must 500 the read");
    assert!(matches!(err, Error::AnyFailure(_)), "got {err:?}");
    assert_eq!(
        ctx.backoff.record_failure(),
        Duration::from_secs(4),
        "a failed read must not arm the backoff (the prime was 1s → 2s, next is 4s)"
    );
}

#[tokio::test]
async fn domain_failed_read_resets_the_backoff_and_the_loop_survives() {
    // A read that fails with a domain error rolls back cleanly: a healthy
    // transaction, so it **resets** the backoff (spec #558 §3.7) — and the
    // loop survives. The canned store reports the part as missing (the domain
    // error); the executor rolls the transaction back and 500s the read.
    let mut conn = MockConn::new();
    conn.expect_rollback().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_read(
        Box::new(mock_session(UserId::from(1))),
        ApiRead::Part(PartId::from(999_999)),
    );
    // The domain reports the part as missing (the domain error): the executor
    // rolls the transaction back and 500s the read.
    let mut facade = MockDomainFacade::new();
    facade.expect_read().returning(|_r, _s, _st| {
        Box::pin(async { Err(Error::NotFound("the part does not exist".to_string())) })
    });
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, facade, MockLane::new(), frames, Box::new(pusher));
    ctx.backoff.record_failure();
    ctx.backoff.record_failure(); // prime: the next pause would be 4s
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a domain-failed read must not end the loop");
    let err = reply
        .await
        .expect("the oneshot resolves")
        .expect_err("a missing part is NotFound");
    assert!(matches!(err, Error::NotFound(_)), "got {err:?}");
    assert_eq!(
        ctx.backoff.record_failure(),
        Duration::from_secs(1),
        "a read transaction that rolled back after a domain error resets the backoff"
    );
}

#[tokio::test]
async fn cleanly_committed_read_resets_the_backoff() {
    // A cleanly committed read is a healthy transaction: it **resets** the
    // backoff (spec #558 §3.7) and resolves the oneshot with the
    // `ReadOutcome`. The canned store returns a user and an empty state
    // (no parts/activities/shops): the `UserState` composite is built from the
    // canned outcomes — the executor's handling (commit + backoff reset) is
    // what is asserted, not the store's behavior.
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) =
        Request::new_read(Box::new(mock_session(UserId::from(1))), ApiRead::User);
    // The canned domain read returns a user and an empty state (no
    // parts/activities/shops): the `UserState` composite is the canned
    // outcome — the executor's handling (commit + backoff reset) is what is
    // asserted, not the domain's behavior.
    let state = UserState {
        user: user_record(UserId::from(1)),
        summary: Summary::default(),
    };
    let mut facade = MockDomainFacade::new();
    facade.expect_read().returning(move |_r, _s, _st| {
        let state = state.clone();
        Box::pin(async move { Ok(ReadOutcome::User(Box::new(state))) })
    });
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, facade, MockLane::new(), frames, Box::new(pusher));
    ctx.backoff.record_failure();
    ctx.backoff.record_failure(); // prime: the next pause would be 4s
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a committed read must not end the loop");
    let outcome = reply
        .await
        .expect("the oneshot resolves")
        .expect("the user read succeeds");
    let ReadOutcome::User(state) = outcome else {
        panic!("the User read returns the UserState composite");
    };
    assert!(
        state.summary.parts.is_empty(),
        "the canned store returns no parts: the summary is empty, got {:?}",
        state.summary.parts
    );
    assert_eq!(
        ctx.backoff.record_failure(),
        Duration::from_secs(1),
        "a cleanly committed read resets the backoff"
    );
}

// --- The loop-side Strava dispatches (spec #558 §3.3/§4) ---

#[tokio::test]
async fn all_stats_round_trips_through_the_loop_transaction() {
    // The admin stats read is dispatched loop-side in the loop's own
    // transaction (spec #558 §3.3): the oneshot resolves with the stats
    // after the commit. The canned store has no Strava users, so the stats are
    // empty.
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_all_stats(Box::new(mock_session(UserId::from(1))));
    // The canned lane has no Strava users, so the stats are empty.
    let mut lane = MockLane::new();
    lane.expect_all_stats().returning(|_st| Ok(Vec::new()));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(
        source,
        MockDomainFacade::new(),
        lane,
        frames,
        Box::new(pusher),
    );
    let result = run_request(&mut ctx, request).await;
    assert!(
        result.is_ok(),
        "a committed stats read must not end the loop"
    );
    let stats = reply
        .await
        .expect("the oneshot resolves")
        .expect("the stats read succeeds");
    assert!(stats.is_empty(), "the canned store has no Strava users");
}

#[tokio::test]
async fn gear_hydration_delivers_the_created_parts_frame() {
    // The hydration write is dispatched loop-side (spec #558 §4): the
    // created parts' union is the frame (delivered on commit, before the
    // oneshot reply) and the reply carries the same summary. The lane is
    // canned to import one bike (returning its created part id); the facade
    // is canned to read the created part back (the frame's entry).
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_gear(Box::new(mock_session(UserId::from(1))));
    // The lane imports one bike (the created part id); the facade reads the
    // created part back (the frame's entry).
    let mut lane = MockLane::new();
    lane.expect_update_gear()
        .returning(|_uid, _st| Ok(vec![PartId::from(1)]));
    let mut facade = MockDomainFacade::new();
    facade
        .expect_part()
        .returning(move |_id, _s, _st| Box::pin(async move { Ok(part_test_bike()) }));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, facade, lane, frames, Box::new(pusher.clone()));
    let result = run_request(&mut ctx, request).await;
    assert!(
        result.is_ok(),
        "a committed hydration must not end the loop"
    );
    let summary = reply
        .await
        .expect("the oneshot resolves")
        .expect("the hydration succeeds");
    // One entry per created part — the one bike from the canned Strava API.
    assert_eq!(
        summary.parts.len(),
        1,
        "the frame is the created parts' union"
    );
    let part = sole_live(&summary.parts);
    assert_eq!(part.name, "Test Bike");
    assert_eq!(part.source.as_deref(), Some("b123"));
    // The frame was delivered on commit, before the reply resolved, with a
    // personal context (the hydration's session has no shop).
    let calls = pusher.calls();
    assert_eq!(
        calls.len(),
        1,
        "the hydration delivers exactly one frame, got {calls:?}"
    );
    assert_eq!(
        calls[0].context,
        DeliveryContext {
            user_id: UserId::from(1),
            shop: None,
            part_owner: None,
        },
        "the hydration's delivery context is personal"
    );
    assert_eq!(
        calls[0].summary, summary,
        "the delivered frame is the created parts' union"
    );
}

#[tokio::test]
async fn failed_gear_arms_the_backoff_like_a_write() {
    // The hydration is a write, not a read (spec #558 §4): a failed `begin`
    // **arms** the backoff — unlike a read (spec #558 §3.7) — and the loop
    // survives. The store is never reached: `begin` fails fast.
    let source = FakeSource::failing();
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_gear(Box::new(mock_session(UserId::from(1))));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(
        source,
        MockDomainFacade::new(),
        MockLane::new(),
        frames,
        Box::new(pusher),
    );
    ctx.backoff.record_failure(); // prime: the next pause would be 2s
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a failed hydration must not end the loop");
    let err = reply
        .await
        .expect("the oneshot resolves")
        .expect_err("a failed begin must 500 the hydration");
    assert!(matches!(err, Error::AnyFailure(_)), "got {err:?}");
    assert_eq!(
        ctx.backoff.record_failure(),
        Duration::from_secs(4),
        "the hydration is a write: a failed begin arms the backoff (1s → 2s → 4s)"
    );
}

#[tokio::test]
async fn gear_hydration_without_new_gear_delivers_an_empty_summary() {
    // The hydration's common case (spec #558 §4): the Strava athlete carries
    // no new gear — `update_gear` creates nothing and the `Summary` is empty.
    // The loop delivers it through the pusher as-is — the empty-`Summary`
    // guard (which pushes nothing) is the pusher's, not the loop's (spec
    // #706). The oneshot resolves with the empty summary. The lane is canned
    // to create nothing; the store only commits.
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_gear(Box::new(mock_session(UserId::from(1))));
    // The lane creates nothing (no new gear): the summary is empty.
    let mut lane = MockLane::new();
    lane.expect_update_gear()
        .returning(|_uid, _st| Ok(Vec::new()));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(
        source,
        MockDomainFacade::new(),
        lane,
        frames,
        Box::new(pusher.clone()),
    );
    let result = run_request(&mut ctx, request).await;
    assert!(
        result.is_ok(),
        "a committed hydration must not end the loop"
    );
    let summary = reply
        .await
        .expect("the oneshot resolves")
        .expect("the hydration succeeds");
    assert!(summary.is_empty(), "nothing new: the summary is empty");
    // The loop delivers the empty summary as-is (the pusher's guard pushes
    // nothing for it).
    let calls = pusher.calls();
    assert_eq!(
        calls.len(),
        1,
        "the loop delivers the summary as-is, got {calls:?}"
    );
    assert!(
        calls[0].summary.is_empty(),
        "the delivered summary is empty"
    );
}

// --- The write's delivery context by the request's session (spec #706) ---

#[tokio::test]
async fn run_request_shop_context_delivers_the_shop_delivery_context() {
    // A write whose **request** session is in a shop context delivers the
    // committed `Summary` through the pusher with the shop's
    // [`DeliveryContext`] — the session's shop and the part owner resolved in
    // the loop's transaction (pre-commit). The routing itself is the
    // pusher's (`Core::deliver`'s tests). The canned domain write returns a
    // summary referencing a part (owned by the session user, 1); the facade
    // resolves that part's owner.
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let pusher = RecordingPusher::new();
    // The write carries the shop context in its own (the request's) session.
    let (request, reply) = Request::new_write(
        Box::new(mock_session_with_shop(UserId::from(1), ShopId::from(7))),
        ApiWrite::PartCreate {
            name: "Shop Chain".to_string(),
            vendor: "Shimano".to_string(),
            model: "CN-HG62".to_string(),
            what: CHAIN,
            purchase: sample_purchase_date(),
        },
    );
    // The canned domain write returns a summary referencing a part; the
    // facade resolves that part's owner (the shop context's delivery input).
    let mut facade = MockDomainFacade::new();
    facade.expect_exec().returning(move |_w, _s, _st| {
        Box::pin(async move { Ok(WriteOutcome::Summary(summary_owned_by(UserId::from(1)))) })
    });
    facade
        .expect_part_owner()
        .returning(|_id, _st| Box::pin(async { Ok(UserId::from(1)) }));

    let mut ctx = test_context(
        source,
        facade,
        MockLane::new(),
        frames,
        Box::new(pusher.clone()),
    );
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a committed write must not end the loop");
    let outcome = reply
        .await
        .expect("the oneshot resolves")
        .expect("the write succeeds");
    let summary = outcome.summary().clone();

    // The loop delivers the summary through the pusher with the shop's
    // context: the session's shop and the resolved part owner.
    let calls = pusher.calls();
    assert_eq!(
        calls.len(),
        1,
        "the write delivers exactly one frame, got {calls:?}"
    );
    assert_eq!(
        calls[0].context,
        DeliveryContext {
            user_id: UserId::from(1),
            shop: Some(ShopId::from(7)),
            part_owner: Some(UserId::from(1)),
        },
        "the delivery context carries the shop and the resolved part owner"
    );
    assert_eq!(
        calls[0].summary, summary,
        "the delivered summary is the write's summary"
    );
}

#[tokio::test]
async fn personal_write_delivers_the_personal_delivery_context() {
    // A write whose **request** session is personal delivers the committed
    // `Summary` through the pusher with a personal [`DeliveryContext`] (no
    // shop, no part owner) — the routing itself is the pusher's. The canned
    // domain write returns a summary referencing a part (owned by the session
    // user, 1); no part owner is resolved (the session has no shop).
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let pusher = RecordingPusher::new();
    // The write carries a personal session (the request's session).
    let (request, reply) = Request::new_write(
        Box::new(mock_session(UserId::from(1))),
        ApiWrite::PartCreate {
            name: "Personal Chain".to_string(),
            vendor: "Shimano".to_string(),
            model: "CN-HG62".to_string(),
            what: CHAIN,
            purchase: sample_purchase_date(),
        },
    );
    // The canned domain write returns a summary referencing a part; no part
    // owner is resolved (the session has no shop).
    let mut facade = MockDomainFacade::new();
    facade.expect_exec().returning(move |_w, _s, _st| {
        Box::pin(async move { Ok(WriteOutcome::Summary(summary_owned_by(UserId::from(1)))) })
    });

    let mut ctx = test_context(
        source,
        facade,
        MockLane::new(),
        frames,
        Box::new(pusher.clone()),
    );
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a committed write must not end the loop");
    let outcome = reply
        .await
        .expect("the oneshot resolves")
        .expect("the write succeeds");
    let summary = outcome.summary().clone();

    // The loop delivers the summary through the pusher with a personal
    // context (no shop, no part owner).
    let calls = pusher.calls();
    assert_eq!(
        calls.len(),
        1,
        "the write delivers exactly one frame, got {calls:?}"
    );
    assert_eq!(
        calls[0].context,
        DeliveryContext {
            user_id: UserId::from(1),
            shop: None,
            part_owner: None,
        },
        "the delivery context is personal (no shop, no part owner)"
    );
    assert_eq!(
        calls[0].summary, summary,
        "the delivered summary is the write's summary"
    );
}
