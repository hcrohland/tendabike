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
    ActTypeId, Activity, ActivityId, ActivityStore, ApiRead, ApiWrite, Attachment, AttachmentStore,
    Error, Notification, NotificationId, NotificationKind, NotificationLifecycle,
    NotificationState, NotificationStore, NotificationSubject, OnboardingStatus, Part, PartId,
    PartNote, PartNoteId, PartNoteStore, PartStore, PartTypeId, ReadOutcome, Service, ServiceId,
    ServicePlan, ServicePlanId, ServicePlanStore, ServiceStore, Shop, ShopId, ShopStore,
    ShopSubscription, Store, SubscriptionId, SubscriptionStatus, Summary, TbResult, Usage, UsageId,
    UsageStore, User, UserId, UserStore, WriteOutcome,
};
use tb_strava::event::Event;
use tb_strava::{StravaDataStore, StravaId, StravaStore, StravaUser};
use time::OffsetDateTime;
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;

use super::{
    DbBackoff, LoopContext, Probe, Select, deliver_frame, next_action, run_request, select_message,
    sleep_or_take_request, stop_remaining,
};
use crate::message::{Request, push_frame};
use crate::{
    Action, ExecutorStatus, FramePusher, Message, Registry, ServiceApi, Txn, TxnSource,
    request_channel,
};

// --- The `mock!` connection witness (issue #631) ---
//
// A single type accumulating the full `Store` seam (the nine sub-traits plus
// the method-free [`Store`] marker) *and* the Strava surface ([`StravaDataStore`]
// plus the method-free [`StravaStore`] marker) *and* the transaction lifecycle
// ([`Txn`]) — i.e. the `TxnSource::Conn: StravaStore + Txn` bound.
//
// `#[async_trait]` on each impl block is the trick that makes the `async fn`
// methods desugar correctly in the generated code (see
// `docs/research/mockall-conn-witness.md` §1). The seam is collision-free
// (post-recipe: renamed methods, `usages_update(&[Usage])`, `stravauser_update_token`
// owning its token), so `mock!`'s flat `expect_<name>` API can express every
// method on one type.
//
// The declared struct name is `Conn`, so `mock!` emits [`MockConn`]; the impl
// blocks reference `Conn` (the macro rewrites it to `MockConn`), and the
// method-free markers `impl Store for Conn` / `impl StravaStore for Conn`
// become `impl … for MockConn`.
//
// Test-support only: this module is `#[cfg(test)]`, so no production `pub`
// surface is widened. `mockall` is a dev-dependency of `tb_exec` only.

mockall::mock! {
    pub Conn {}

    #[async_trait::async_trait]
    impl PartStore for Conn {
        async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part>;
        async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>>;
        async fn part_create(
            &mut self,
            what: PartTypeId,
            name: String,
            vendor: String,
            model: String,
            purchase: OffsetDateTime,
            source: Option<String>,
            usage: UsageId,
            owner: UserId,
            shop: Option<ShopId>,
        ) -> TbResult<Part>;
        async fn part_update(&mut self, part: Part) -> TbResult<Part>;
        async fn part_delete(&mut self, part: PartId) -> TbResult<PartId>;
        async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize>;
        async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>>;
        async fn parts_register_shop(
            &mut self,
            shop_id: ShopId,
            part_id: Vec<PartId>,
        ) -> TbResult<Vec<Part>>;
        async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>>;
        async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>>;
    }

    #[async_trait::async_trait]
    impl UserStore for Conn {
        async fn user_get(&mut self, uid: UserId) -> TbResult<User>;
        async fn user_create(
            &mut self,
            firstname: &str,
            lastname: &str,
            avatar: &Option<String>,
        ) -> TbResult<User>;
        async fn user_update(
            &mut self,
            uid: &UserId,
            firstname: &str,
            lastname: &str,
            avatar: &Option<String>,
        ) -> TbResult<User>;
        async fn user_delete(&mut self, user: &UserId) -> TbResult<usize>;
        async fn update_onboarding_status(
            &mut self,
            uid: &UserId,
            status: OnboardingStatus,
        ) -> TbResult<User>;
    }

    #[async_trait::async_trait]
    impl ShopStore for Conn {
        async fn shop_create(
            &mut self,
            name: String,
            description: Option<String>,
            auto_approve: bool,
            owner: UserId,
        ) -> TbResult<Shop>;
        async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop>;
        async fn shop_update(
            &mut self,
            id: ShopId,
            name: String,
            description: Option<String>,
            auto_approve: bool,
        ) -> TbResult<Shop>;
        async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize>;
        async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>>;
        async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>>;
        async fn subscription_create(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
            message: Option<String>,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription>;
        async fn subscription_find_active(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<Option<ShopSubscription>>;
        async fn subscription_find_pending(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<Option<ShopSubscription>>;
        async fn subscription_update_status(
            &mut self,
            id: SubscriptionId,
            status: SubscriptionStatus,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_approve(
            &mut self,
            id: SubscriptionId,
            status: SubscriptionStatus,
            response_message: Option<String>,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()>;
        async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>>;
        async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>>;
    }

    #[async_trait::async_trait]
    impl ActivityStore for Conn {
        async fn activity_create(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>>;
        async fn activity_update(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize>;
        async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize>;
        async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>>;
        async fn activities_find_by_gear_and_time(
            &mut self,
            part: PartId,
            begin: OffsetDateTime,
            end: OffsetDateTime,
        ) -> TbResult<Vec<Activity>>;
        async fn get_by_user_and_time(
            &mut self,
            uid: UserId,
            rstart: OffsetDateTime,
        ) -> TbResult<Activity>;
        async fn activity_set_gear_if_null(
            &mut self,
            user: UserId,
            types: Vec<ActTypeId>,
            partid: &PartId,
        ) -> TbResult<Vec<Activity>>;
        async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>>;
    }

    #[async_trait::async_trait]
    impl AttachmentStore for Conn {
        async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment>;
        async fn attachment_delete(&mut self, att: Attachment) -> TbResult<Attachment>;
        async fn attachments_delete_by_parts(&mut self, parts: &[Part]) -> TbResult<usize>;
        async fn attachment_get_by_gear_and_time(
            &mut self,
            act_gear: PartId,
            start: OffsetDateTime,
        ) -> TbResult<Vec<Attachment>>;
        async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>>;
        async fn attachment_get_by_part_and_time(
            &mut self,
            pid: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn assembly_get_by_types_time_and_gear(
            &mut self,
            types: Vec<PartTypeId>,
            gear: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Vec<Attachment>>;
        async fn attachment_find_part_of_type_at_hook_and_time(
            &mut self,
            what: PartTypeId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_successor(
            &mut self,
            part_id: PartId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
            what: PartTypeId,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_later_attachment_for_part(
            &mut self,
            part_id: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_part_attached_already(
            &mut self,
            part_id: PartId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
    }

    #[async_trait::async_trait]
    impl PartNoteStore for Conn {
        async fn partnote_create_text(
            &mut self,
            part: PartId,
            name: String,
            created: OffsetDateTime,
        ) -> TbResult<PartNote>;
        async fn partnote_create_file(
            &mut self,
            part: PartId,
            name: String,
            mime: String,
            filename: Option<String>,
            size: i64,
            data: Vec<u8>,
            created: OffsetDateTime,
        ) -> TbResult<PartNote>;
        async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>>;
        async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote>;
        async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>>;
        async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote>;
        async fn partnote_update_file(
            &mut self,
            id: PartNoteId,
            name: String,
            mime: String,
            filename: Option<String>,
            size: i64,
            data: Vec<u8>,
        ) -> TbResult<PartNote>;
        async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote>;
        async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId>;
    }

    #[async_trait::async_trait]
    impl UsageStore for Conn {
        async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>>;
        async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize>;
        async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage>;
        async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize>;
        async fn delete_all(&mut self) -> TbResult<usize>;
    }

    #[async_trait::async_trait]
    impl ServiceStore for Conn {
        async fn service_create(&mut self, service: Service) -> TbResult<Service>;
        async fn service_get(&mut self, service: ServiceId) -> TbResult<Service>;
        async fn service_update(&mut self, service: Service) -> TbResult<Service>;
        async fn service_delete(&mut self, service: ServiceId) -> TbResult<usize>;
        async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize>;
        async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>>;
    }

    #[async_trait::async_trait]
    impl ServicePlanStore for Conn {
        async fn serviceplan_create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan>;
        async fn serviceplan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_delete(&mut self, plan: ServicePlanId) -> TbResult<usize>;
        async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize>;
        async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>>;
        async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>>;
    }

    #[async_trait::async_trait]
    impl NotificationStore for Conn {
        async fn notification_create(
            &mut self,
            recipient: UserId,
            lifecycle: NotificationLifecycle,
            kind: NotificationKind,
            subject: NotificationSubject,
            message: Option<String>,
        ) -> TbResult<Notification>;
        async fn notification_read(
            &mut self,
            id: NotificationId,
            recipient: UserId,
        ) -> TbResult<Option<Notification>>;
        async fn notification_read_all(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;
        async fn notification_all_for_user(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;
    }

    impl Store for Conn {}

    #[async_trait::async_trait]
    impl StravaDataStore for Conn {
        async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>>;
        async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser>;
        async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>>;
        async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser>;
        async fn stravauser_update_token(
            &mut self,
            stravaid: StravaId,
            refresh: Option<String>,
        ) -> TbResult<StravaUser>;
        async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize>;
        async fn strava_event_create(&mut self, e: Event) -> TbResult<()>;
        async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()>;
        async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()>;
        async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>>;
        async fn strava_event_get_later(
            &mut self,
            obj_id: i64,
            oid: StravaId,
        ) -> TbResult<Vec<Event>>;
        async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;
        async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64>;
        async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize>;
    }

    impl StravaStore for Conn {}

    #[async_trait::async_trait]
    impl Txn for Conn {
        async fn commit(self) -> TbResult<()>;
        async fn rollback(self) -> TbResult<()>;
    }
}

/// A `TxnSource` whose `begin` fails (the DB-down corner) or hands out the
/// test's configured [`MockConn`] — the store the executor's per-request
/// dispatch drives against canned outcomes. `begin` is called exactly once
/// per `run_request` (one loop-owned transaction), so the mock is taken out
/// of the `Mutex` on that call.
#[derive(Clone)]
struct FakeSource {
    begin_fails: bool,
    conn: Arc<Mutex<Option<MockConn>>>,
}

impl FakeSource {
    /// A source whose `begin` fails with a `DatabaseFailure` (the DB-down
    /// corner): no store is ever handed out.
    fn failing() -> Self {
        Self {
            begin_fails: true,
            conn: Arc::new(Mutex::new(None)),
        }
    }

    /// A source that hands out `conn` on its (single) `begin`.
    fn with(conn: MockConn) -> Self {
        Self {
            begin_fails: false,
            conn: Arc::new(Mutex::new(Some(conn))),
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
        self.conn.lock().unwrap().take().ok_or_else(|| {
            Error::DatabaseFailure(anyhow::anyhow!("begin was called more than once"))
        })
    }
}

/// A future that never resolves: stands in for the Strava queue probe being
/// blocked (a slow or busy queue), so the select has a second, non-ready
/// branch without a store.
struct Never;

impl Future for Never {
    type Output = Option<Probe>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

/// A future that immediately resolves with a ready probe outcome.
async fn ready_probe(probe: Probe) -> Option<Probe> {
    Some(probe)
}

fn request() -> (Request, oneshot::Receiver<TbResult<WriteOutcome>>) {
    Request::new_write(
        Box::new(mock_session(UserId::from(1))),
        ApiWrite::UserOnboardingPostpone,
    )
}

/// The context the per-request tests drive: the loop's wiring (source,
/// frames, pusher) with the context's starting state (a fresh backoff, the
/// first-idle flag set). The consumed-event channel, the re-probe cadence,
/// and the wake signal are never touched by `run_request`, so they are fresh
/// dummies.
fn test_context(
    source: FakeSource,
    frames: broadcast::Sender<Summary>,
    pusher: Box<dyn FramePusher>,
) -> LoopContext<FakeSource> {
    LoopContext::new(
        source,
        UserId::from(1),
        frames,
        pusher,
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

// --- Context-based delivery (spec #517, ticket #584) ---
//
// The executor routes a committed `Summary` by the session's `shop()` value:
// a user context pushes to the executor's own personal stream; a shop context
// pushes to the shop stream and the part owner's personal stream (never the
// executor's own stream). `deliver_frame` is the routing unit; the tests
// exercise it directly (full control over the context and the part owner) and
// through `run_request` (the session's `shop()` feeds the routing).

/// A `FramePusher` test double: records the `push_to_shop`/`push_to_user`
/// calls (target + summary) so the delivery routing is observable. `Clone`
/// shares the recording (the `Arc`), so a test keeps an observation handle
/// after moving the pusher into the context.
#[derive(Default, Clone)]
struct RecordingPusher {
    calls: std::sync::Arc<std::sync::Mutex<Vec<PushCall>>>,
}

/// One recorded push: the target (a shop stream or a user's personal stream)
/// and the frame it carried.
#[derive(Debug, Clone, PartialEq)]
enum PushCall {
    /// A push to a shop stream.
    Shop(ShopId, Summary),
    /// A push to a user's personal stream.
    User(UserId, Summary),
}

impl RecordingPusher {
    fn new() -> Self {
        Self::default()
    }

    /// The recorded pushes (a clone: the test reads them after the loop).
    fn calls(&self) -> Vec<PushCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl FramePusher for RecordingPusher {
    async fn push_to_shop(&self, shop_id: ShopId, summary: Summary) {
        self.calls
            .lock()
            .unwrap()
            .push(PushCall::Shop(shop_id, summary));
    }

    async fn push_to_user(&self, user_id: UserId, summary: Summary) {
        self.calls
            .lock()
            .unwrap()
            .push(PushCall::User(user_id, summary));
    }
}

/// A part owned by `owner` (the routing's `push_to_user` target).
fn part_owned_by(owner: UserId) -> Part {
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

/// A canned `Shop` owned by `owner` (the `subscribe_shop` ownership read).
fn shop_owned_by(owner: UserId) -> Shop {
    Shop {
        id: ShopId::from(7),
        owner,
        name: "Test Shop".to_string(),
        description: None,
        auto_approve: true,
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// A non-empty `Summary` with a single part owned by `owner`: the frame a
/// shop-context write would deliver.
fn summary_owned_by(owner: UserId) -> Summary {
    let mut summary = Summary::default();
    summary.parts.upsert(part_owned_by(owner));
    summary
}

/// A non-empty `Summary` with a service referencing a part (no `Part` in the
/// summary — the common shape for service operations): the frame a
/// shop-context service write would deliver. The part owner is not derivable
/// from this summary; the executor resolves it via the store.
fn summary_with_service() -> Summary {
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
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
    conn.expect_partid_get_part()
        .returning(|_pid| Err(Error::NotFound("the part does not exist".to_string())));
    conn.expect_rollback().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_read(
        Box::new(mock_session(UserId::from(1))),
        ApiRead::Part(PartId::from(999_999)),
    );
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
    conn.expect_user_get()
        .returning(move |_| Ok(user_record(UserId::from(1))));
    conn.expect_get_all().returning(|_uid| Ok(Vec::new()));
    conn.expect_shops_get_all_for_user()
        .returning(|_uid| Ok(Vec::new()));
    conn.expect_part_get_all_for_userid()
        .returning(|_uid| Ok(Vec::new()));
    conn.expect_by_user().returning(|_uid| Ok(Vec::new()));
    conn.expect_notification_all_for_user()
        .returning(|_uid| Ok(Vec::new()));
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) =
        Request::new_read(Box::new(mock_session(UserId::from(1))), ApiRead::User);
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
    conn.expect_stravauser_get_all()
        .returning(|| Ok(Vec::new()));
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, _frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_all_stats(Box::new(mock_session(UserId::from(1))));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
async fn gear_hydration_pushes_the_created_parts_frame() {
    // The hydration write is dispatched loop-side (spec #558 §4): the
    // created parts' union is the frame (published on commit, before the
    // oneshot reply) and the reply carries the same summary. The Strava API is
    // canned in the thread-local map the thin free functions consult (one
    // bike); the store is canned on the `MockConn` (no existing part for the
    // source, the create, the read-back).
    tb_strava::test_support::begin_test();
    tb_strava::test_support::seed_token_cache(
        UserId::from(1),
        StravaId::from(42),
        "test-access-token",
    );
    tb_strava::test_support::queue("/athlete", r#"{"bikes":[{"id":"b123"}],"shoes":[]}"#);
    tb_strava::test_support::queue(
        "/gear/b123",
        r#"{"id":"b123","name":"Test Bike","brand_name":"TestBrand","model_name":"M1","frame_type":1}"#,
    );
    let mut conn = MockConn::new();
    // The part is not yet imported (twice: before and after the Strava gear
    // fetch), so `into_partid` creates it; the read-back returns the created
    // part (the frame's entry).
    conn.expect_partid_get_by_source().returning(|_s| Ok(None));
    conn.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _purchase, _source, _usage, _owner, _shop| {
            Ok(part_test_bike())
        },
    );
    conn.expect_partid_get_part()
        .returning(move |_pid| Ok(part_test_bike()));
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, mut frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_gear(Box::new(mock_session(UserId::from(1))));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
    // The frame was published on commit, before the reply resolved.
    let frame = frame_rx
        .try_recv()
        .expect("the created parts push exactly one frame");
    assert_eq!(frame, summary);
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
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
async fn gear_hydration_without_new_gear_pushes_no_frame() {
    // The hydration's common case (spec #558 §4): the Strava athlete carries
    // no new gear — `update_gear` creates nothing, the `Summary` is empty,
    // and `push_frame` sends **nothing**: exactly today's behavior. The
    // oneshot resolves with the empty summary. The Strava API is canned in the
    // thread-local map (an empty athlete); the store only commits.
    tb_strava::test_support::begin_test();
    tb_strava::test_support::seed_token_cache(
        UserId::from(1),
        StravaId::from(42),
        "test-access-token",
    );
    tb_strava::test_support::queue("/athlete", r#"{"bikes":[],"shoes":[]}"#);
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, mut frame_rx) = broadcast::channel(4);
    let (request, reply) = Request::new_gear(Box::new(mock_session(UserId::from(1))));
    let pusher = RecordingPusher::new();
    let mut ctx = test_context(source, frames, Box::new(pusher));
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
    assert!(
        frame_rx.try_recv().is_err(),
        "no new gear: an empty summary pushes no frame"
    );
}

// --- The write's frame routing by the request's session context (spec #517) ---

#[tokio::test]
async fn run_request_shop_context_routes_via_pusher_not_own_stream() {
    // A write whose **request** session is in a shop context routes the
    // committed `Summary`'s frame through the pusher (shop stream + part
    // owner) and NOT the executor's own `frames` channel — the frame's
    // delivery keys on the request's session. The canned store creates the
    // part (owned by the session user, 1) and resolves the part owner for the
    // `push_to_user` target.
    let mut conn = MockConn::new();
    conn.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _purchase, _source, _usage, _owner, _shop| {
            Ok(part_owned_by(UserId::from(1)))
        },
    );
    conn.expect_partid_get_part()
        .returning(move |_pid| Ok(part_owned_by(UserId::from(1))));
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, mut frame_rx) = broadcast::channel(4);
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

    let mut ctx = test_context(source, frames, Box::new(pusher.clone()));
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a committed write must not end the loop");
    let outcome = reply
        .await
        .expect("the oneshot resolves")
        .expect("the write succeeds");
    let summary = outcome.summary().clone();

    // The executor's own (shop owner's) personal stream gets nothing.
    assert!(
        frame_rx.try_recv().is_err(),
        "a shop-context write must not push to the executor's own stream"
    );
    // The pusher routes to the shop stream and the part owner (the session
    // user, 1).
    let calls = pusher.calls();
    assert!(
        calls.iter().any(|call| {
            matches!(
                call,
                PushCall::Shop(sid, s) if *sid == ShopId::from(7) && *s == summary
            )
        }),
        "the shop stream receives the frame, got {calls:?}"
    );
    assert!(
        calls.iter().any(|call| {
            matches!(
                call,
                PushCall::User(uid, s) if *uid == UserId::from(1) && *s == summary
            )
        }),
        "the part owner (the session user) receives the frame, got {calls:?}"
    );
}

#[tokio::test]
async fn personal_write_routes_to_own_stream_not_pusher() {
    // A write whose **request** session is personal routes the committed
    // `Summary`'s frame to the executor's own `frames` channel (the personal
    // stream) and NOT through the pusher (no shop stream, no part-owner push)
    // — the frame's delivery keys on the request's session. The canned store
    // creates the part (owned by the session user, 1).
    let mut conn = MockConn::new();
    conn.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _purchase, _source, _usage, _owner, _shop| {
            Ok(part_owned_by(UserId::from(1)))
        },
    );
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    let (frames, mut frame_rx) = broadcast::channel(4);
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

    let mut ctx = test_context(source, frames, Box::new(pusher.clone()));
    let result = run_request(&mut ctx, request).await;
    assert!(result.is_ok(), "a committed write must not end the loop");
    let outcome = reply
        .await
        .expect("the oneshot resolves")
        .expect("the write succeeds");
    let summary = outcome.summary().clone();

    // The frame goes to the executor's own personal stream.
    let frame = frame_rx
        .try_recv()
        .expect("a personal-context write pushes to the executor's own stream");
    assert_eq!(frame, summary);
    // And NOT through the pusher (no shop stream, no part-owner push).
    assert!(
        pusher.calls().is_empty(),
        "a personal-context write must not route through the pusher, got {:?}",
        pusher.calls()
    );
}

// --- Shop stream infrastructure (spec #517, ticket #582) ---
//
// The `Registry` owns a per-shop `broadcast` channel map (the shop streams),
// a `FramePusher` (`push_to_shop`/`push_to_user`), and `ServiceApi::
// subscribe_shop` (owner-only, on-demand). The tests drive the `Registry`
// through its public surface (`subscribe_shop`, `push_to_shop`,
// `push_to_user`, `shop_open`); the store is a canned [`MockConn`] whose
// `shop_get` reports the shop's owner (the `subscribe_shop` ownership read).

/// A `TxnSource` whose `subscribe_shop` ownership read is canned: the store
/// reports `shop_get` → a shop owned by `owner`, so an owner's subscribe
/// succeeds and a non-owner's is `Forbidden`.
fn shop_source(owner: UserId) -> FakeSource {
    let mut conn = MockConn::new();
    conn.expect_shop_get()
        .returning(move |_id| Ok(shop_owned_by(owner)));
    conn.expect_commit().returning(|| Ok(()));
    FakeSource::with(conn)
}

/// A non-empty `Summary` (a frame the shop/user streams should deliver).
fn non_empty_summary() -> Summary {
    let mut summary = Summary::default();
    summary.usages.upsert(Usage::default());
    summary
}

#[tokio::test]
async fn subscribe_shop_rejects_a_non_owner() {
    // A non-owner is forbidden (the web layer maps this to a 403): the canned
    // store reports the shop's owner as 1, so user 2 is rejected.
    let registry = Registry::new(shop_source(UserId::from(1)), Duration::from_secs(60));

    let result = registry
        .subscribe_shop(UserId::from(2), ShopId::from(7))
        .await;
    assert!(
        matches!(result, Err(Error::Forbidden(_))),
        "a non-owner is forbidden, got {result:?}"
    );
}

#[tokio::test]
async fn shop_stream_lifecycle_create_receive_drop() {
    let registry = Registry::new(shop_source(UserId::from(1)), Duration::from_secs(60));
    let shop_id = ShopId::from(7);

    // Created on demand: no stream before the owner's first connect.
    assert!(
        !registry.shop_open(shop_id).await,
        "no shop stream before a connect"
    );

    // The owner's connect opens the channel (the canned store reports the
    // owner as the session user).
    let mut rx = registry
        .subscribe_shop(UserId::from(1), shop_id)
        .await
        .expect("the owner can subscribe");
    assert!(
        registry.shop_open(shop_id).await,
        "the channel is open while a receiver is attached"
    );

    // A frame pushed to the shop stream reaches the subscriber.
    let summary = non_empty_summary();
    registry.push_to_shop(shop_id, summary.clone()).await;
    let got = rx.recv().await.expect("the subscriber receives the frame");
    assert_eq!(got, summary);

    // Dropping the last receiver closes the channel: the "open shop" check
    // (receiver_count() > 0) is no longer met, so the entry is reaped.
    drop(rx);
    assert!(
        !registry.shop_open(shop_id).await,
        "the channel is dropped when the last receiver disconnects"
    );
}

#[tokio::test]
async fn push_to_shop_is_a_noop_without_a_subscriber() {
    let registry = Registry::new(shop_source(UserId::from(1)), Duration::from_secs(60));
    let shop_id = ShopId::from(7);

    // No subscriber: the push is dropped (no panic, no receiver to send to).
    registry.push_to_shop(shop_id, non_empty_summary()).await;

    // And after a subscriber disconnects, the push is dropped too (the
    // reaped channel has no receiver, so the send is a no-op).
    let rx = registry
        .subscribe_shop(UserId::from(1), shop_id)
        .await
        .expect("the owner can subscribe");
    drop(rx);
    registry.push_to_shop(shop_id, non_empty_summary()).await;
}

#[tokio::test]
async fn push_to_user_delivers_to_a_running_executor() {
    // No store is involved: `push_to_user` feeds the user's executor frame
    // channel, so the source's mock is never `begin`-ed.
    let registry = Registry::new(FakeSource::failing(), Duration::from_secs(60));
    let mut frame_rx = registry
        .install_executor_for_test(UserId::from(1), ExecutorStatus::Running)
        .await;

    let summary = non_empty_summary();
    registry
        .push_to_user(UserId::from(1), summary.clone())
        .await;

    let got = frame_rx
        .try_recv()
        .expect("the running executor's frame channel receives the frame");
    assert_eq!(got, summary);
}

#[tokio::test]
async fn push_to_user_is_a_noop_without_a_running_executor() {
    // No store is involved: `push_to_user` is a no-op with no (or a reaped)
    // executor, so the source's mock is never `begin`-ed.
    let registry = Registry::new(FakeSource::failing(), Duration::from_secs(60));

    // No executor for the user: a no-op (no panic, no receiver).
    registry
        .push_to_user(UserId::from(1), non_empty_summary())
        .await;

    // A reaped executor: a no-op (the frame channel is not fed).
    let mut frame_rx = registry
        .install_executor_for_test(UserId::from(2), ExecutorStatus::Reaped)
        .await;
    registry
        .push_to_user(UserId::from(2), non_empty_summary())
        .await;
    assert!(
        frame_rx.try_recv().is_err(),
        "a reaped executor receives no frame"
    );
}

// --- Context-based delivery: the five scenarios (spec #517, ticket #584) ---

#[tokio::test]
async fn deliver_user_context_to_personal_stream() {
    // AC2: a user context (`shop == None`) delivers to the executor's own
    // personal stream (the existing `push_frame` behavior, unchanged) — and
    // never to a shop or user stream through the pusher.
    let pusher = RecordingPusher::new();
    let (frames, mut frame_rx) = broadcast::channel(4);
    let summary = summary_owned_by(UserId::from(1));

    deliver_frame(&pusher, &frames, UserId::from(1), None, None, &summary).await;

    let got = frame_rx
        .try_recv()
        .expect("the user's personal stream gets the frame");
    assert_eq!(got, summary);
    assert!(
        pusher.calls().is_empty(),
        "a user context pushes nothing through the pusher, got {:?}",
        pusher.calls()
    );
}

#[tokio::test]
async fn deliver_shop_context_to_shop_stream() {
    // AC3: a shop context delivers to the shop stream via `push_to_shop`.
    let pusher = RecordingPusher::new();
    let (frames, _frame_rx) = broadcast::channel(4);
    let shop_id = ShopId::from(7);
    let summary = summary_owned_by(UserId::from(2));

    deliver_frame(
        &pusher,
        &frames,
        UserId::from(1),
        Some(shop_id),
        Some(UserId::from(2)),
        &summary,
    )
    .await;

    assert!(
        pusher
            .calls()
            .iter()
            .any(|call| matches!(call, PushCall::Shop(sid, s) if *sid == shop_id && *s == summary)),
        "the shop stream receives the frame, got {:?}",
        pusher.calls()
    );
}

#[tokio::test]
async fn deliver_shop_context_to_part_owner() {
    // AC4: a shop context delivers to the part owner's personal stream via
    // `push_to_user` (the single-owner Summary: the first part's owner).
    let pusher = RecordingPusher::new();
    let (frames, _frame_rx) = broadcast::channel(4);
    let shop_id = ShopId::from(7);
    let part_owner = UserId::from(2);
    let summary = summary_owned_by(part_owner);

    deliver_frame(
        &pusher,
        &frames,
        UserId::from(1),
        Some(shop_id),
        Some(part_owner),
        &summary,
    )
    .await;

    assert!(
        pusher.calls().iter().any(
            |call| matches!(call, PushCall::User(uid, s) if *uid == part_owner && *s == summary)
        ),
        "the part owner's personal stream receives the frame, got {:?}",
        pusher.calls()
    );
}

#[tokio::test]
async fn deliver_shop_context_not_to_shop_owner() {
    // AC5: a shop context does NOT deliver to the shop owner's personal
    // stream (the executor's own `frames` channel) when the shop owner is not
    // the part owner — the frame goes only to the shop stream and the part
    // owner. Here the part owner (2) is not the shop owner (the executor's
    // user, 1).
    let pusher = RecordingPusher::new();
    let (frames, mut frame_rx) = broadcast::channel(4);
    let shop_id = ShopId::from(7);
    let summary = summary_owned_by(UserId::from(2));

    deliver_frame(
        &pusher,
        &frames,
        UserId::from(1),
        Some(shop_id),
        Some(UserId::from(2)),
        &summary,
    )
    .await;

    // The executor's own (shop owner's) personal stream gets nothing.
    assert!(
        frame_rx.try_recv().is_err(),
        "the shop owner's personal stream must not receive the frame"
    );
    // The pusher never targets the shop owner (1) — only the part owner (2).
    let calls = pusher.calls();
    assert!(
        !calls
            .iter()
            .any(|call| matches!(call, PushCall::User(uid, _) if *uid == UserId::from(1))),
        "the shop owner (1) is not the part owner, so no push targets them, got {calls:?}"
    );
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, PushCall::User(uid, _) if *uid == UserId::from(2))),
        "the part owner (2) is targeted, got {calls:?}"
    );
}

#[tokio::test]
async fn deliver_shop_context_to_shop_owner_when_part_owner() {
    // AC6: a shop context delivers to the shop owner's personal stream when
    // the shop owner IS the part owner — the `push_to_user(shop_owner)` call
    // targets the shop owner's own executor stream.
    let pusher = RecordingPusher::new();
    let (frames, _frame_rx) = broadcast::channel(4);
    let shop_id = ShopId::from(7);
    let shop_owner = UserId::from(2);
    let summary = summary_owned_by(shop_owner); // part owner == shop owner

    deliver_frame(
        &pusher,
        &frames,
        UserId::from(1),
        Some(shop_id),
        Some(shop_owner),
        &summary,
    )
    .await;

    assert!(
        pusher.calls().iter().any(
            |call| matches!(call, PushCall::User(uid, s) if *uid == shop_owner && *s == summary)
        ),
        "the shop owner (the part owner) receives the frame via push_to_user, got {:?}",
        pusher.calls()
    );
}

#[tokio::test]
async fn deliver_shop_context_service_summary_to_part_owner() {
    // The common shop-context shape: a service operation produces a summary
    // with a service (referencing a part) but no `Part` entity. The part
    // owner is resolved by the executor (via the store) and passed to
    // `deliver_frame` — the summary alone does not carry it.
    let pusher = RecordingPusher::new();
    let (frames, _frame_rx) = broadcast::channel(4);
    let shop_id = ShopId::from(7);
    let part_owner = UserId::from(2);
    let summary = summary_with_service();

    // The summary references a part (via the service) but does not contain
    // the part itself — `referenced_part` finds it.
    assert_eq!(
        summary.referenced_part(),
        Some(PartId::from(1)),
        "the service's part_id is the referenced part"
    );

    deliver_frame(
        &pusher,
        &frames,
        UserId::from(1),
        Some(shop_id),
        Some(part_owner),
        &summary,
    )
    .await;

    assert!(
        pusher.calls().iter().any(
            |call| matches!(call, PushCall::User(uid, s) if *uid == part_owner && *s == summary)
        ),
        "the part owner's personal stream receives the frame, got {:?}",
        pusher.calls()
    );
    assert!(
        pusher
            .calls()
            .iter()
            .any(|call| matches!(call, PushCall::Shop(sid, s) if *sid == shop_id && *s == summary)),
        "the shop stream also receives the frame, got {:?}",
        pusher.calls()
    );
}

// --- Cross-recipient notification routing (issue #677) ---

/// A `Summary` carrying a notification for `recipient`.
fn summary_with_notification(recipient: UserId) -> Summary {
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

/// A user-context write that produces a notification for a different user
/// pushes it to that user's personal stream via `push_to_user` (the acting
/// user's own notifications ride the normal frame).
#[tokio::test]
async fn deliver_cross_recipient_notification_to_user_stream() {
    let pusher = RecordingPusher::new();
    let (frames, mut frame_rx) = broadcast::channel(4);
    let acting_user = UserId::from(1);
    let recipient = UserId::from(3);
    let summary = summary_with_notification(recipient);

    deliver_frame(&pusher, &frames, acting_user, None, None, &summary).await;

    // The acting user's normal frame carries the full summary.
    let got = frame_rx.try_recv().expect("the acting user gets the frame");
    assert_eq!(got, summary);
    // The cross-recipient notification is pushed to the recipient's stream.
    let calls = pusher.calls();
    assert!(
        calls.iter().any(
            |call| matches!(call, PushCall::User(uid, s) if *uid == recipient && *s == summary)
        ),
        "the recipient's personal stream receives the notification, got {:?}",
        calls
    );
}

/// A user-context write that produces a notification for the acting user
/// does NOT push it again via `push_to_user` (it already rides the normal
/// frame).
#[tokio::test]
async fn deliver_no_cross_recipient_for_acting_user() {
    let pusher = RecordingPusher::new();
    let (frames, mut frame_rx) = broadcast::channel(4);
    let acting_user = UserId::from(1);
    let summary = summary_with_notification(acting_user);

    deliver_frame(&pusher, &frames, acting_user, None, None, &summary).await;

    // The acting user's normal frame carries the notification.
    let got = frame_rx.try_recv().expect("the acting user gets the frame");
    assert_eq!(got, summary);
    // No `push_to_user` call for the acting user (no duplicate).
    assert!(
        pusher.calls().is_empty(),
        "no push_to_user for the acting user's own notification, got {:?}",
        pusher.calls()
    );
}

/// A shop-context write that produces a notification for the part owner does
/// NOT push it again via `push_to_user` (it already rides the normal frame
/// which is pushed to the part owner's personal stream).
#[tokio::test]
async fn deliver_no_cross_recipient_for_part_owner_in_shop_context() {
    let pusher = RecordingPusher::new();
    let (frames, _frame_rx) = broadcast::channel(4);
    let acting_user = UserId::from(1);
    let part_owner = UserId::from(2);
    let shop_id = ShopId::from(7);
    let summary = summary_with_notification(part_owner);

    deliver_frame(
        &pusher,
        &frames,
        acting_user,
        Some(shop_id),
        Some(part_owner),
        &summary,
    )
    .await;

    let calls = pusher.calls();
    // The shop stream gets the frame.
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, PushCall::Shop(sid, s) if *sid == shop_id && *s == summary)),
        "the shop stream receives the frame, got {:?}",
        calls
    );
    // The part owner gets the frame via the normal shop-context routing.
    assert!(
        calls.iter().any(
            |call| matches!(call, PushCall::User(uid, s) if *uid == part_owner && *s == summary)
        ),
        "the part owner receives the frame, got {:?}",
        calls
    );
    // Exactly one `push_to_user` call (the part owner), no duplicate.
    let user_pushes: Vec<_> = calls
        .iter()
        .filter(|call| matches!(call, PushCall::User(_, _)))
        .collect();
    assert_eq!(
        user_pushes.len(),
        1,
        "exactly one push_to_user (the part owner), got {:?}",
        calls
    );
}

/// A shop-context write that produces a notification for a third party (neither
/// the acting user nor the part owner) pushes it to that user's personal stream.
#[tokio::test]
async fn deliver_cross_recipient_third_party_in_shop_context() {
    let pusher = RecordingPusher::new();
    let (frames, _frame_rx) = broadcast::channel(4);
    let acting_user = UserId::from(1);
    let part_owner = UserId::from(2);
    let third_party = UserId::from(5);
    let shop_id = ShopId::from(7);
    let summary = summary_with_notification(third_party);

    deliver_frame(
        &pusher,
        &frames,
        acting_user,
        Some(shop_id),
        Some(part_owner),
        &summary,
    )
    .await;

    let calls = pusher.calls();
    // The third party gets the notification via `push_to_user`.
    assert!(
        calls.iter().any(
            |call| matches!(call, PushCall::User(uid, s) if *uid == third_party && *s == summary)
        ),
        "the third party's personal stream receives the notification, got {:?}",
        calls
    );
}
