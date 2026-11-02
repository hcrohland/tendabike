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

//! The registry's lifecycle core (spec #558 §6, split from the [`super`]
//! facade, spec #697): the deep "how does an executor stay alive" machinery —
//! the per-user executor map and the per-shop frame-channel map, the
//! per-executor handle, the settled-fate [`ExecutorStatus`], and the
//! spawn-on-first-demand / reap-when-no-stream / respawn logic, the wake
//! signal, the stop-all teardown, the shop-stream lifecycle, and the
//! frame-pusher surface.
//!
//! The core is **source-agnostic**: it owns the two maps and the lifecycle,
//! but takes the [`TxnSource`] as a parameter to its spawn (the
//! [`super::Registry`] facade owns the source and the `Box<dyn
//! FramePusher>` it hands the loop). It is crate-internal — the facade
//! re-exports [`ExecutorStatus`] and delegates its lifecycle calls here, so
//! the crate's public surface is unchanged and no new public type is
//! introduced.
//!
//! The two maps keep their tight, independent lock scopes: an ownership read
//! never holds the shop-map lock, and a shop push never holds the executor
//! lock. The executor's fate is shared through a `watch` channel (a
//! `JoinHandle` is not `Clone`, so it cannot be handed to both the core and
//! the stream tasks): the spawned task records [`ExecutorStatus::Reaped`] on
//! a clean idle exit and [`ExecutorStatus::Dead`] on a lifecycle error or
//! panic. A clone of the frame sender is kept alive in the task until
//! *after* that record is written, so a stream that sees the frame channel
//! close already sees the settled status.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use log::debug;
use tokio::sync::{Mutex, Notify, broadcast, watch};

use tb_domain::{ShopId, Summary, TbResult, UserId};
use tb_strava::event::Event as StravaEvent;

use crate::LoopContext;
use crate::TxnSource;
use crate::message::RequestSender;
use crate::{registry::FramePusher, request_channel};

/// The per-user frame channel's capacity: a burst of frames for a lagged
/// stream is dropped (the client's reconnect + full refresh covers it), so a
/// modest bound is enough.
const BROADCAST_CAPACITY: usize = 64;

/// The executor task's settled fate, shared with its streams (a `JoinHandle`
/// is not `Clone`, so it cannot be handed to both the registry and the
/// stream tasks). `Running` is the initial value; the task records `Reaped`
/// on a clean idle exit and `Dead` on a lifecycle error, and a watchdog
/// records `Dead` if the task panics (a panic skips the task's own record).
///
/// The web layer names it in the stream teardown (spec #558 §6.3): the
/// unfold must distinguish a clean reap (keep the connection, heartbeat,
/// re-subscribe) from a dead executor (end the stream so the client
/// reconnects and respawns) — an opaque fate cannot carry that dichotomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutorStatus {
    /// The loop is running (or has not yet recorded its exit).
    Running,
    /// The loop exited cleanly (`run` returned `Ok(())`) — always with no
    /// stream attached (an attached stream keeps the loop alive): a stream
    /// that met the loop in the connect-time race re-subscribes (respawning
    /// the executor) and settles.
    Reaped,
    /// The loop ended with a lifecycle error, or the task panicked: the
    /// stream closes so the client reconnects and respawns (spec §4.5).
    Dead,
}

/// One live per-user executor: the shared fate channel, the send half of the
/// user's request channel (the write and read handlers enqueue here,
/// spec #558 §3.2), the SSE frame sink the stream tasks subscribe to, the
/// consumed-Strava-event channel the admin sync awaits (spec §6.4), the
/// in-memory wake signal the webhook fires after queueing a Strava event
/// (spec §4.4/§4.6), and the task's abort handle (the core's `stop_all`
/// tears the executor down — rescan, issue #446). The `AbortHandle` (not
/// the `JoinHandle`) is stored: the watchdog keeps the original handle to
/// `await` and record `Dead` on a panic **or** the abort `JoinError`. The
/// fields the [`super::Registry`] facade's `exec_field` closures reach into
/// (`status`, `request_sender`, `frames`, `events`) are `pub(super)` so the
/// facade can pull them; the rest stay private to the core.
#[derive(Clone)]
pub(super) struct Executor {
    pub(super) status: watch::Receiver<ExecutorStatus>,
    /// The request handlers' door into this user's loop (spec #558 §3.2):
    /// enqueue a `Request` here and await its oneshot.
    pub(super) request_sender: RequestSender,
    pub(super) frames: broadcast::Sender<Summary>,
    /// Every Strava event the loop consumed (spec §6.4): the admin sync's
    /// completion signal, correlated by the event's identity.
    pub(super) events: broadcast::Sender<StravaEvent>,
    /// The in-memory wake signal (spec §4.4/§4.6): fired by the core's
    /// `wake` after a Strava event is queued; the running loop selects on
    /// it, so the event is processed at once, even mid-idle-sleep.
    wake: Arc<Notify>,
    /// Aborts the executor task (`stop_all`): the watchdog records `Dead`
    /// on the abort `JoinError`, like a panic.
    task: tokio::task::AbortHandle,
}

/// The per-shop frame channel with its owner (the shop stream is owner-only,
/// ticket #583). The owner is the caller `subscribe_shop` just verified as
/// the shop's owner, so the tag is never stale: a stream exists only while
/// a receiver is attached, and every subscription re-verifies. The tag is
/// what lets a notification frame fan out to every open shop stream of its
/// recipient (follow-up to issue #687) — the recipient's shop views show
/// all their notifications (the shop-scoped hydration snapshot loads them
/// all), so a dismiss in one tab must clear the others.
struct ShopStream {
    sender: broadcast::Sender<Summary>,
    owner: UserId,
}

/// The routing inputs a [`Core::deliver`] needs: the acting user, the
/// session's shop (if any), and the resolved part owner. The loop constructs
/// it from its existing values; the pusher consumes it. It is a smaller,
/// delivery-specific projection — not the full acting session (the routing
/// policy needs only these three fields).
// `allow(dead_code)`: the loop's migration (spec #706) is the production
// consumer; until then only the tests construct it.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(super) struct DeliveryContext {
    /// The acting user (the caller's identity).
    pub(super) user_id: UserId,
    /// The session's shop (a shop context) or `None` (a personal context).
    pub(super) shop: Option<ShopId>,
    /// The resolved part owner (the loop resolves it in its transaction,
    /// pre-commit — the pusher does not resolve it).
    pub(super) part_owner: Option<UserId>,
}

/// The registry's lifecycle core (spec #697): the per-user executor map and
/// the per-shop frame-channel map, the spawn-on-first-demand /
/// reap-when-no-stream / respawn policy, the wake signal, the stop-all
/// teardown, the shop-stream lifecycle, and the frame-pusher surface.
/// Source-agnostic — the [`super::Registry`] facade owns the [`TxnSource`]
/// (and the `Box<dyn FramePusher>` it hands the loop) and passes both to the
/// spawn. Its methods are `pub(super)` so the facade can delegate to them.
#[derive(Clone)]
pub(super) struct Core {
    executors: Arc<Mutex<HashMap<UserId, Executor>>>,
    /// The per-shop frame channels (spec #517, ticket #582): a standalone
    /// `broadcast` channel per shop, created on demand by `subscribe_shop`
    /// and reaped when its last receiver disconnects (the "open shop" check
    /// is implicit: `receiver_count() > 0`). A separate mutex from the
    /// executor map keeps the two lock scopes tight (an ownership read never
    /// holds the shop-map lock, and a shop push never holds the executor
    /// lock).
    shop_streams: Arc<Mutex<HashMap<ShopId, ShopStream>>>,
    idle_timeout: Duration,
}

impl Core {
    /// The core with an explicit idle cadence (the construction site is the
    /// facade's `new`/`with_defaults`; the maps start empty). The SSE
    /// heartbeat cadence is the crate constant [`super::HEARTBEAT_INTERVAL`]
    /// — the `KeepAlive` wiring uses it directly.
    pub(super) fn new(idle_timeout: Duration) -> Self {
        Self {
            executors: Arc::new(Mutex::new(HashMap::new())),
            shop_streams: Arc::new(Mutex::new(HashMap::new())),
            idle_timeout,
        }
    }

    /// Fire the wake signal for the user's executor (spec §4.4/§4.6): a
    /// pure signal to a **running** executor — it interrupts the loop's
    /// idle wait so a just-queued Strava event is processed at once. It
    /// never spawns an executor: a reaped (or not-yet-spawned) executor
    /// picks the event up on its next spawn (an SSE connect or an API
    /// write) — the DB queue is the source of truth. Infallible by design:
    /// a missed wake only delays the event to the next spawn, so the
    /// webhook treats it as non-critical.
    pub(super) async fn wake(&self, user_id: UserId) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            exec.wake.notify_one();
        }
    }

    /// Stop every live executor (the admin rescan's global re-hydration,
    /// issue #446): for each executor, abort the task (the watchdog records
    /// `Dead` on the abort `JoinError`, like a panic) — the aborted task
    /// drops its clones of the frame and consumed-event senders, and
    /// clearing the map drops the core's, so both channels close and
    /// open streams see the close and end (their clients reconnect and the
    /// catch-up snapshot re-hydrates them; an in-flight admin sync hits its
    /// already-handled queue-fallback path) — then clear the map. The next
    /// `subscribe`/`write` respawns fresh (`ensure_running`'s existing
    /// behavior). The lock is held across the loop, so no executor can be
    /// spawned between the aborts and the clear.
    pub(super) async fn stop_all(&self) {
        let mut map = self.executors.lock().await;
        for exec in map.values() {
            exec.task.abort();
        }
        map.clear();
    }

    /// A subscription to the user's executor's consumed-Strava-event channel
    /// (spec §6.4): spawning the executor if it is not running. The admin
    /// sync subscribes **before** it enqueues its event, so the completion
    /// signal cannot be missed.
    pub(super) async fn events<S>(
        &self,
        source: &S,
        user_id: UserId,
        pusher: Box<dyn FramePusher>,
    ) -> TbResult<broadcast::Receiver<StravaEvent>>
    where
        S: TxnSource + Clone + 'static,
    {
        self.exec_field(source, user_id, pusher, |exec| exec.events.subscribe())
            .await
    }

    /// Push a frame to the user's SSE streams (test helper: the production
    /// path pushes frames from the loop via `tb_exec::push_frame`).
    pub(super) async fn send_frame(&self, user_id: UserId, summary: Summary) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id) {
            let _ = exec.frames.send(summary);
        }
    }

    /// The user's executor's current fate (test helper).
    pub(super) async fn status(&self, user_id: UserId) -> Option<ExecutorStatus> {
        let map = self.executors.lock().await;
        map.get(&user_id).map(|exec| *exec.status.borrow())
    }

    /// Whether `shop_id`'s stream is open (test helper): the channel exists
    /// and has a receiver (the "open shop" check, spec #517). Dead channels
    /// (no receiver) are reaped on the way, so this is the observable form
    /// of "the channel is dropped when its last receiver disconnects".
    #[cfg(test)]
    pub(super) async fn shop_open(&self, shop_id: ShopId) -> bool {
        let mut map = self.shop_streams.lock().await;
        Self::reap_shop_streams(&mut map);
        map.contains_key(&shop_id)
    }

    /// Test helper: install a synthetic executor for `user_id` (bypassing
    /// [`Self::ensure_running`], which needs a Strava session the unit fakes
    /// don't have) and return a receiver on its frame channel. The
    /// executor's fate is `status`. The task handle is a completed task (its
    /// `AbortHandle` is valid; `abort` on it is a no-op), so no task leaks.
    #[cfg(test)]
    pub(super) async fn install_executor_for_test(
        &self,
        user_id: UserId,
        status: ExecutorStatus,
    ) -> broadcast::Receiver<Summary> {
        let (status_tx, status_rx) = watch::channel(status);
        let (request_sender, _request_receiver) = request_channel();
        let (frames, frame_rx) = broadcast::channel(BROADCAST_CAPACITY);
        let (events, _events_rx) = broadcast::channel(BROADCAST_CAPACITY);
        let wake = Arc::new(Notify::new());
        let task = tokio::spawn(async {}).abort_handle();
        // The stored receiver keeps the watch channel open; drop the sender.
        drop(status_tx);
        let exec = Executor {
            status: status_rx,
            request_sender,
            frames,
            events,
            wake,
            task,
        };
        self.executors.lock().await.insert(user_id, exec);
        frame_rx
    }

    /// Create the shop channel on demand and return a subscription to it
    /// (spec #517, ticket #582; the "open shop" check is implicit:
    /// `receiver_count() > 0`). The owner tag is the caller the facade's
    /// `subscribe_shop` just verified as the shop's owner (follow-up to
    /// issue #687): it lets a notification frame fan out to every open shop
    /// stream of its recipient. This method does only the channel creation
    /// (no store call — the ownership read stays in the facade, ADR-0010).
    pub(super) async fn subscribe_shop(
        &self,
        shop_id: ShopId,
        owner: UserId,
    ) -> broadcast::Receiver<Summary> {
        let mut map = self.shop_streams.lock().await;
        Self::reap_shop_streams(&mut map);
        let stream = map.entry(shop_id).or_insert_with(|| ShopStream {
            sender: broadcast::channel(BROADCAST_CAPACITY).0,
            owner,
        });
        stream.sender.subscribe()
    }

    /// Push to `shop_id`'s stream (spec #517): reap the dead channels, then
    /// send on the shop channel if it exists (after the reap, an existing
    /// entry has a receiver, so the send is not dropped).
    pub(super) async fn push_to_shop(&self, shop_id: ShopId, summary: Summary) {
        let mut map = self.shop_streams.lock().await;
        Self::reap_shop_streams(&mut map);
        if let Some(stream) = map.get(&shop_id) {
            let _ = stream.sender.send(summary);
        }
    }

    /// Push to `user_id`'s personal stream (spec #517): send on the user's
    /// executor's frame channel if the executor is running; a no-op
    /// otherwise (no executor, or a settled one).
    pub(super) async fn push_to_user(&self, user_id: UserId, summary: Summary) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            let _ = exec.frames.send(summary);
        }
    }

    /// Push to every open shop stream `user_id` owns (follow-up to issue
    /// #687): reap the dead channels, then send on each channel whose owner
    /// tag is `user_id`, skipping the shops already delivered to.
    pub(super) async fn push_to_recipient_shops(
        &self,
        user_id: UserId,
        summary: Summary,
        skip: &[ShopId],
    ) {
        let mut map = self.shop_streams.lock().await;
        Self::reap_shop_streams(&mut map);
        for (shop_id, stream) in map.iter() {
            if stream.owner == user_id && !skip.contains(shop_id) {
                let _ = stream.sender.send(summary.clone());
            }
        }
    }

    /// Route a committed [`Summary`] to its delivery targets given the acting
    /// session's [`DeliveryContext`] (the complete frame-routing policy, moved
    /// from the executor's `deliver_frame`, spec #705): the empty-`Summary`
    /// guard, the personal-vs-shop context, the shops-the-operation-touched
    /// fan-out, the cross-recipient notification routing, the recipient
    /// shop-stream fan-out, and the shop-context personal-stream catch-up.
    /// The channel-sending is the [`Self::push_to_shop`]/[`Self::push_to_user`]/
    /// [`Self::push_to_recipient_shops`] helpers (unchanged — they stay
    /// `pub(super)` so the [`super::Registry`] facade's `FramePusher` impl
    /// keeps working; the trait collapses in a later ticket).
    // `allow(dead_code)`: the loop's migration (spec #706) is the production
    // consumer; until then only the tests call it.
    #[allow(dead_code)]
    pub(super) async fn deliver(&self, context: DeliveryContext, summary: Summary) {
        if summary.is_empty() {
            return;
        }
        let DeliveryContext {
            user_id,
            shop,
            part_owner,
        } = context;
        // Push the normal frame to the acting user's delivery target(s).
        match shop {
            None => self.push_to_user(user_id, summary.clone()).await,
            Some(shop_id) => {
                self.push_to_shop(shop_id, summary.clone()).await;
                if let Some(owner) = part_owner {
                    self.push_to_user(owner, summary.clone()).await;
                }
            }
        }
        // Shops the operation touched (issue #687): routed by the summary's
        // content, not the session's context. The session's own shop was
        // already delivered above.
        for shop_entry in summary.shops.live() {
            if Some(shop_entry.id) != shop {
                self.push_to_shop(shop_entry.id, summary.clone()).await;
            }
        }
        // Cross-recipient notification routing (issue #677): a
        // notifications-only frame to each other recipient's personal stream
        // (and the shop stream, for a recipient "in" the shop).
        for notification in summary.notifications.live() {
            let recipient = notification.recipient;
            if recipient == user_id {
                continue;
            }
            if let Some(owner) = part_owner
                && recipient == owner
            {
                continue;
            }
            let mut recipient_summary = Summary::default();
            recipient_summary.notifications.upsert(notification);
            self.push_to_user(recipient, recipient_summary.clone())
                .await;
            if let Some(shop_id) = shop {
                self.push_to_shop(shop_id, recipient_summary).await;
            }
        }
        // The recipient's shop streams (follow-up to issue #687): each
        // recipient's open shop streams get a notifications-only frame — the
        // shop-scoped snapshot filters parts to the current shop, so a full
        // summary would pollute the other tabs' parts maps. A tombstone is
        // always the acting user's own (the recipient guard: only the
        // recipient can read a notification).
        let notifications = summary.notifications.live();
        let mut recipients: Vec<UserId> = Vec::new();
        for notification in &notifications {
            if !recipients.contains(&notification.recipient) {
                recipients.push(notification.recipient);
            }
        }
        let mut notification_only = Summary::default();
        notification_only
            .notifications
            .merge(summary.notifications.clone());
        if summary.notifications.len() > notifications.len() && !recipients.contains(&user_id) {
            recipients.push(user_id);
        }
        // Shops already delivered (the session's own, the summary's): no
        // double frame on the same channel.
        let mut skip: Vec<ShopId> = summary.shops.live().iter().map(|s| s.id).collect();
        if let Some(shop_id) = shop {
            skip.push(shop_id);
        }
        for recipient in &recipients {
            self.push_to_recipient_shops(*recipient, notification_only.clone(), &skip)
                .await;
        }
        // Shop context: the acting user's own notification state also reaches
        // their personal stream (not the normal delivery target there) —
        // unless the part owner push above already delivered it.
        if shop.is_some() && recipients.contains(&user_id) && part_owner != Some(user_id) {
            self.push_to_user(user_id, summary).await;
        }
    }

    /// Reap shop channels whose last receiver has disconnected (the "open
    /// shop" check is implicit: `receiver_count() > 0`). Called on every
    /// shop-map access (`subscribe_shop`, `push_to_shop`, `shop_open`) so a
    /// dead channel does not linger in the map.
    fn reap_shop_streams(map: &mut HashMap<ShopId, ShopStream>) {
        map.retain(|_, stream| stream.sender.receiver_count() > 0);
    }

    /// Lock the core, ensure the user's executor is live (spawning it if it
    /// is absent or settled), and pull one field from it: the shared shape
    /// of `subscribe`, `events`, and `write`. The lock is dropped
    /// when this returns, before the caller's next await. The source and the
    /// `Box<dyn FramePusher>` the loop routes committed `Summary`s through
    /// are the facade's (the core is source-agnostic and does not own a
    /// pusher).
    pub(super) async fn exec_field<S, R>(
        &self,
        source: &S,
        user_id: UserId,
        pusher: Box<dyn FramePusher>,
        f: impl FnOnce(&Executor) -> R,
    ) -> TbResult<R>
    where
        S: TxnSource + Clone + 'static,
    {
        let mut map = self.executors.lock().await;
        let exec =
            Self::ensure_running(&mut map, source, user_id, self.idle_timeout, pusher).await?;
        Ok(f(&exec))
    }

    /// Return the user's live executor, spawning it if it is absent or has
    /// settled (reaped/dead): build the loop's identity-only session (its
    /// shop is irrelevant — the per-request shop rides on each request), open
    /// a fresh request channel and frame channel, and spawn the `tb_exec`
    /// loop.
    async fn ensure_running<S>(
        map: &mut HashMap<UserId, Executor>,
        source: &S,
        user_id: UserId,
        idle_timeout: Duration,
        pusher: Box<dyn FramePusher>,
    ) -> TbResult<Executor>
    where
        S: TxnSource + Clone + 'static,
    {
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            return Ok(exec.clone());
        }

        let (request_sender, reader) = request_channel();
        let (frames, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (status_tx, status_rx) = watch::channel(ExecutorStatus::Running);
        let wake = Arc::new(Notify::new());
        debug!("spawning the executor for user {user_id}");

        // The task keeps a clone of the frame sender alive until *after* it
        // records its fate, so a stream that sees the frame channel close has
        // already seen the settled status (no race between the two signals).
        // `source`, `frames_for_run`, and `events_for_run` are owned clones:
        // the `async move` block needs `'static` values, not the `&S`
        // reference.
        let owned_source = source.clone();
        let frames_for_run = frames.clone();
        let events_for_run = events.clone();
        let wake_for_run = wake.clone();
        let watchdog_status = status_tx.clone();
        let join = tokio::spawn(async move {
            let result = LoopContext::new(
                owned_source,
                user_id,
                frames_for_run,
                pusher,
                events_for_run,
                idle_timeout,
                wake_for_run,
            )
            .run(reader)
            .await;
            let _ = status_tx.send(match result {
                Ok(()) => ExecutorStatus::Reaped,
                Err(_) => ExecutorStatus::Dead,
            });
            result
        });
        // A panicked task skips the `status_tx.send` above; the watchdog
        // records the panic as `Dead` (like any other failure) so
        // `ensure_running` respawns instead of handing out a dead executor
        // forever (spec §4.4: the next SSE connect or API request respawns
        // it). The watchdog lives exactly as long as the executor task.
        //
        // The abort handle is taken before the watchdog moves `join`: it is
        // how `stop_all` tears the task down (rescan, issue #446) — the
        // watchdog's `join.await` then sees the abort `JoinError` and
        // records `Dead`, exactly like a panic.
        let task = join.abort_handle();
        tokio::spawn(async move {
            if join.await.is_err() {
                let _ = watchdog_status.send(ExecutorStatus::Dead);
            }
        });

        let exec = Executor {
            status: status_rx,
            request_sender,
            frames,
            events,
            wake,
            task,
        };
        map.insert(user_id, exec.clone());
        Ok(exec)
    }
}

#[cfg(test)]
mod tests {
    //! The lifecycle core's own tests (spec #697, ticket #699): the core is
    //! constructed **directly** (no `Registry` facade, no `ServiceApi` in the
    //! way) and its lifecycle methods are driven at the single
    //! transaction-witness seam (the `MockConn`-backed `FakeSource`) — "how
    //! does an executor stay alive", verifiable in isolation. The core's maps
    //! and channels are real; only the store is faked. The test double and
    //! fixtures are the executor module's (widened to `pub(crate)`), imported
    //! here — not copied.

    use std::time::Duration;

    use tb_domain::test_support::fixtures::sample_purchase_date;
    use tb_domain::{
        Notification, NotificationId, NotificationKind, NotificationLifecycle, NotificationState,
        NotificationSubject, PartId, ShopId, Summary, TbResult, UserId,
    };
    use uuid::Uuid;

    use super::{Core, DeliveryContext, ExecutorStatus};
    use crate::executor::executor_tests::{
        FakeSource, RecordingPusher, non_empty_summary, part_owned_by, shop_owned_by,
        summary_owned_by, summary_with_notification, summary_with_service,
    };
    use crate::registry::FramePusher;

    /// A canned `FramePusher` for the core's spawn (the `RecordingPusher`
    /// double): the core is source-agnostic and takes the pusher as a
    /// parameter, so a test double stands in for the facade.
    fn pusher() -> Box<dyn FramePusher> {
        Box::new(RecordingPusher::new())
    }

    /// `exec_field` spawns the user's executor on the first demand (the
    /// "spawn on first demand" half of the lifecycle) and reports it
    /// `Running`. The store is the canned `FakeSource` (its `begin` fails, so
    /// the loop's queue probe backs off and the executor stays `Running`
    /// without a real database).
    #[tokio::test]
    async fn exec_field_spawns_the_executor_on_first_demand() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);
        let source = FakeSource::failing();

        core.exec_field(&source, user_id, pusher(), |_| ()).await?;

        assert_eq!(
            core.status(user_id).await,
            Some(ExecutorStatus::Running),
            "the first demand spawns the executor (Running)"
        );
        Ok(())
    }

    /// `stop_all` tears every executor down (abort + clear the map) and the
    /// next demand respawns it fresh: the "stop-all teardown" and "respawn"
    /// halves of the lifecycle. The map entry is gone after `stop_all`
    /// (`status` → `None`); a fresh `exec_field` brings it back `Running`.
    #[tokio::test]
    async fn stop_all_tears_down_and_the_next_demand_respawns() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);

        core.exec_field(&FakeSource::failing(), user_id, pusher(), |_| ())
            .await?;
        assert_eq!(core.status(user_id).await, Some(ExecutorStatus::Running));

        core.stop_all().await;
        assert_eq!(
            core.status(user_id).await,
            None,
            "stop_all clears the executor map"
        );

        // The next demand respawns fresh.
        core.exec_field(&FakeSource::failing(), user_id, pusher(), |_| ())
            .await?;
        assert_eq!(
            core.status(user_id).await,
            Some(ExecutorStatus::Running),
            "the next demand respawns the executor (Running)"
        );
        Ok(())
    }

    /// `wake` is a pure signal to a **running** executor: it never spawns one
    /// (the DB queue is the source of truth, spec §4.4/§4.6), so a wake on a
    /// core with no live executor leaves the map empty.
    #[tokio::test]
    async fn wake_never_spawns_an_executor() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);

        core.wake(user_id).await;

        assert_eq!(
            core.status(user_id).await,
            None,
            "a wake never spawns an executor"
        );
        Ok(())
    }

    /// `wake` on a running executor is a no-op on the settled fate: it
    /// interrupts the loop's idle wait (so a just-queued event is processed at
    /// once) but never spawns a second executor nor changes the fate.
    #[tokio::test]
    async fn wake_on_a_running_executor_is_a_pure_signal() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);
        let source = FakeSource::failing();

        core.exec_field(&source, user_id, pusher(), |_| ()).await?;
        core.wake(user_id).await;

        assert_eq!(
            core.status(user_id).await,
            Some(ExecutorStatus::Running),
            "a wake leaves a running executor running"
        );
        Ok(())
    }

    /// `subscribe_shop` opens the shop channel on demand and `push_to_shop`
    /// delivers a frame to the open stream (the shop-stream lifecycle,
    /// spec #517). No store is involved — the ownership read stays in the
    /// facade — so the source is never `begin`-ed.
    #[tokio::test]
    async fn push_to_shop_delivers_to_an_open_shop_stream() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);

        let mut rx = core.subscribe_shop(shop_id, UserId::from(1)).await;
        let summary = non_empty_summary();
        core.push_to_shop(shop_id, summary.clone()).await;

        let got = rx
            .recv()
            .await
            .expect("the open shop stream receives the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// `push_to_shop` is a no-op with no subscriber (the "open shop" check is
    /// implicit: `receiver_count() > 0`): no panic, and a bare push opens no
    /// shop stream.
    #[tokio::test]
    async fn push_to_shop_is_a_noop_without_a_subscriber() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);

        core.push_to_shop(shop_id, non_empty_summary()).await;

        assert!(
            !core.shop_open(shop_id).await,
            "a push with no subscriber opens no shop stream"
        );
        Ok(())
    }

    /// `push_to_user` delivers a frame to a running executor's personal stream
    /// (the user-stream push, spec #517): the core's synthetic-executor
    /// installer stands in for a live executor, and no store is involved.
    #[tokio::test]
    async fn push_to_user_delivers_to_a_running_executor() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);
        let mut frame_rx = core
            .install_executor_for_test(user_id, ExecutorStatus::Running)
            .await;

        let summary = non_empty_summary();
        core.push_to_user(user_id, summary.clone()).await;

        let got = frame_rx
            .try_recv()
            .expect("the running executor's frame channel receives the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// `push_to_user` is a no-op for a settled (reaped) executor: the frame
    /// channel is not fed (spec #517).
    #[tokio::test]
    async fn push_to_user_is_a_noop_for_a_reaped_executor() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);
        let mut frame_rx = core
            .install_executor_for_test(user_id, ExecutorStatus::Reaped)
            .await;

        core.push_to_user(user_id, non_empty_summary()).await;

        assert!(
            frame_rx.try_recv().is_err(),
            "a reaped executor receives no frame"
        );
        Ok(())
    }

    // --- The routing policy: `Core::deliver` (spec #705) ---
    //
    // The routing behavior now drives `Core::deliver` directly: a `Core` is
    // constructed, its maps are populated with test channels (the executor's
    // frame channel via `install_executor_for_test`, the shop streams via
    // `subscribe_shop`), `deliver` is called with a [`DeliveryContext`] and a
    // `Summary`, and the assertions check which channels received which
    // frames — external behavior, not which internal helper was called. The
    // fixtures are the executor module's (widened to `pub(crate)`), imported
    // here — not copied.

    /// A [`DeliveryContext`] for the routing tests.
    fn context(
        user_id: UserId,
        shop: Option<ShopId>,
        part_owner: Option<UserId>,
    ) -> DeliveryContext {
        DeliveryContext {
            user_id,
            shop,
            part_owner,
        }
    }

    /// A user context (`shop == None`) delivers to the acting user's personal
    /// stream — and nothing else.
    #[tokio::test]
    async fn deliver_user_context_to_personal_stream() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);
        let mut acting_rx = core
            .install_executor_for_test(user_id, ExecutorStatus::Running)
            .await;
        let summary = summary_owned_by(user_id);

        core.deliver(context(user_id, None, None), summary.clone())
            .await;

        let got = acting_rx
            .try_recv()
            .expect("the acting user's personal stream gets the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// A shop context delivers to the shop stream.
    #[tokio::test]
    async fn deliver_shop_context_to_shop_stream() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);
        let mut shop_rx = core.subscribe_shop(shop_id, UserId::from(1)).await;
        let summary = summary_owned_by(UserId::from(2));

        core.deliver(
            context(UserId::from(1), Some(shop_id), Some(UserId::from(2))),
            summary.clone(),
        )
        .await;

        let got = shop_rx
            .try_recv()
            .expect("the shop stream receives the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// A shop context delivers to the part owner's personal stream.
    #[tokio::test]
    async fn deliver_shop_context_to_part_owner() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);
        let part_owner = UserId::from(2);
        let mut owner_rx = core
            .install_executor_for_test(part_owner, ExecutorStatus::Running)
            .await;
        let summary = summary_owned_by(part_owner);

        core.deliver(
            context(UserId::from(1), Some(shop_id), Some(part_owner)),
            summary.clone(),
        )
        .await;

        let got = owner_rx
            .try_recv()
            .expect("the part owner's personal stream receives the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// A shop context does NOT deliver to the shop owner's personal stream
    /// when the shop owner is not the part owner — the frame goes only to the
    /// shop stream and the part owner. Here the acting user (1) is the shop
    /// owner, and the part owner is (2).
    #[tokio::test]
    async fn deliver_shop_context_not_to_shop_owner() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);
        let shop_owner = UserId::from(1);
        let part_owner = UserId::from(2);
        let mut shop_owner_rx = core
            .install_executor_for_test(shop_owner, ExecutorStatus::Running)
            .await;
        let mut part_owner_rx = core
            .install_executor_for_test(part_owner, ExecutorStatus::Running)
            .await;
        let summary = summary_owned_by(part_owner);

        core.deliver(
            context(shop_owner, Some(shop_id), Some(part_owner)),
            summary.clone(),
        )
        .await;

        // The shop owner's personal stream gets nothing.
        assert!(
            shop_owner_rx.try_recv().is_err(),
            "the shop owner's personal stream must not receive the frame"
        );
        // The part owner's personal stream gets the frame.
        let got = part_owner_rx
            .try_recv()
            .expect("the part owner's personal stream receives the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// A shop context delivers to the shop owner's personal stream when the
    /// shop owner IS the part owner.
    #[tokio::test]
    async fn deliver_shop_context_to_shop_owner_when_part_owner() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);
        let shop_owner = UserId::from(2);
        let mut shop_owner_rx = core
            .install_executor_for_test(shop_owner, ExecutorStatus::Running)
            .await;
        let summary = summary_owned_by(shop_owner); // part owner == shop owner

        core.deliver(
            context(UserId::from(1), Some(shop_id), Some(shop_owner)),
            summary.clone(),
        )
        .await;

        let got = shop_owner_rx
            .try_recv()
            .expect("the shop owner (the part owner) receives the frame");
        assert_eq!(got, summary);
        Ok(())
    }

    /// The common shop-context shape: a service operation produces a summary
    /// with a service (referencing a part) but no `Part` entity. The part
    /// owner is resolved by the loop and carried in the context — the summary
    /// alone does not carry it.
    #[tokio::test]
    async fn deliver_shop_context_service_summary_to_part_owner() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);
        let part_owner = UserId::from(2);
        let mut shop_rx = core.subscribe_shop(shop_id, UserId::from(1)).await;
        let mut owner_rx = core
            .install_executor_for_test(part_owner, ExecutorStatus::Running)
            .await;
        let summary = summary_with_service();

        core.deliver(
            context(UserId::from(1), Some(shop_id), Some(part_owner)),
            summary.clone(),
        )
        .await;

        let got_owner = owner_rx
            .try_recv()
            .expect("the part owner's personal stream receives the frame");
        assert_eq!(got_owner, summary);
        let got_shop = shop_rx
            .try_recv()
            .expect("the shop stream receives the frame");
        assert_eq!(got_shop, summary);
        Ok(())
    }

    /// A user-context write that produces a notification for a different user
    /// pushes it to that user's personal stream (the acting user's own
    /// notifications ride the normal frame).
    #[tokio::test]
    async fn deliver_cross_recipient_notification_to_user_stream() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let recipient = UserId::from(3);
        let mut acting_rx = core
            .install_executor_for_test(acting_user, ExecutorStatus::Running)
            .await;
        let mut recipient_rx = core
            .install_executor_for_test(recipient, ExecutorStatus::Running)
            .await;
        let summary = summary_with_notification(recipient);

        core.deliver(context(acting_user, None, None), summary.clone())
            .await;

        // The acting user's normal frame carries the full summary.
        let got = acting_rx
            .try_recv()
            .expect("the acting user gets the frame");
        assert_eq!(got, summary);
        // The cross-recipient notification reaches the recipient's stream.
        let got_recipient = recipient_rx
            .try_recv()
            .expect("the recipient's personal stream receives the notification");
        assert_eq!(got_recipient, summary);
        Ok(())
    }

    /// A user-context write (the acting session has no shop) whose summary
    /// carries a part registered in a shop pushes the frame to that shop's
    /// stream (issue #687): the shop owner's open shop view listens on the
    /// shop stream, and the session's context is the actor's, not the shop's.
    #[tokio::test]
    async fn deliver_cross_recipient_notification_to_shop_stream_without_shop_context()
    -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let shop_owner = UserId::from(2);
        let shop_id = ShopId::from(7);
        let mut shop_rx = core.subscribe_shop(shop_id, shop_owner).await;
        let mut part = part_owned_by(acting_user);
        part.shop = Some(shop_id);
        let mut summary = Summary::default();
        summary.parts.upsert(part);
        summary.shops.upsert(shop_owned_by(shop_owner));
        let notification = Notification {
            id: NotificationId::from(Uuid::now_v7()),
            recipient: shop_owner,
            lifecycle: NotificationLifecycle::Transient,
            kind: NotificationKind::PartRegistered,
            subject: NotificationSubject::Part(PartId::from(1)),
            message: None,
            state: NotificationState::New,
            created_at: sample_purchase_date(),
        };
        summary.notifications.upsert(notification.clone());

        core.deliver(context(acting_user, None, None), summary.clone())
            .await;

        // The shop stream receives a frame carrying the notification (the shop
        // the operation touched is routed by the summary's content).
        let got = shop_rx
            .try_recv()
            .expect("the shop stream receives the frame");
        assert!(
            got.notifications
                .live()
                .iter()
                .any(|n| n.id == notification.id),
            "the shop stream's frame carries the cross-recipient notification, got {got:?}"
        );
        Ok(())
    }

    /// A shop-context write whose summary carries the session's own shop does
    /// not push a duplicate frame to the shop stream (the session's shop is
    /// already delivered by the normal routing; the summary's shop is
    /// skipped).
    #[tokio::test]
    async fn deliver_shop_context_no_duplicate_shop_push() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let shop_id = ShopId::from(7);
        let part_owner = UserId::from(2);
        let mut shop_rx = core.subscribe_shop(shop_id, UserId::from(1)).await;
        let mut summary = summary_owned_by(part_owner);
        summary.shops.upsert(shop_owned_by(UserId::from(1)));

        core.deliver(
            context(UserId::from(1), Some(shop_id), Some(part_owner)),
            summary.clone(),
        )
        .await;

        let got = shop_rx
            .try_recv()
            .expect("the shop stream receives the frame once");
        assert_eq!(got, summary);
        assert!(
            shop_rx.try_recv().is_err(),
            "the shop stream receives the frame exactly once, no duplicate"
        );
        Ok(())
    }

    /// A user-context write that produces a notification for the acting user
    /// does NOT push it again to the acting user's personal stream (it already
    /// rides the normal frame).
    #[tokio::test]
    async fn deliver_no_cross_recipient_for_acting_user() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let mut acting_rx = core
            .install_executor_for_test(acting_user, ExecutorStatus::Running)
            .await;
        let summary = summary_with_notification(acting_user);

        core.deliver(context(acting_user, None, None), summary.clone())
            .await;

        // The acting user's personal stream gets exactly one frame (the normal
        // frame), no duplicate cross-recipient push.
        let got = acting_rx
            .try_recv()
            .expect("the acting user gets the frame");
        assert_eq!(got, summary);
        assert!(
            acting_rx.try_recv().is_err(),
            "no duplicate push to the acting user's own notification"
        );
        Ok(())
    }

    /// A shop-context write that produces a notification for the part owner
    /// does NOT push it again to the part owner's personal stream (it already
    /// rides the normal frame which is pushed to the part owner's personal
    /// stream).
    #[tokio::test]
    async fn deliver_no_cross_recipient_for_part_owner_in_shop_context() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let part_owner = UserId::from(2);
        let shop_id = ShopId::from(7);
        let mut shop_rx = core.subscribe_shop(shop_id, acting_user).await;
        let mut owner_rx = core
            .install_executor_for_test(part_owner, ExecutorStatus::Running)
            .await;
        let summary = summary_with_notification(part_owner);

        core.deliver(
            context(acting_user, Some(shop_id), Some(part_owner)),
            summary.clone(),
        )
        .await;

        // The part owner's personal stream gets exactly one frame (the normal
        // shop-context push), no duplicate cross-recipient push.
        let got_owner = owner_rx
            .try_recv()
            .expect("the part owner receives the frame");
        assert_eq!(got_owner, summary);
        assert!(
            owner_rx.try_recv().is_err(),
            "exactly one push to the part owner, no duplicate"
        );
        // The shop stream gets the frame.
        let got_shop = shop_rx
            .try_recv()
            .expect("the shop stream receives the frame");
        assert_eq!(got_shop, summary);
        Ok(())
    }

    /// A shop-context write that produces a notification for a third party
    /// (neither the acting user nor the part owner) pushes it to that user's
    /// personal stream and to the shop stream (so a recipient "in" the shop
    /// receives it).
    #[tokio::test]
    async fn deliver_cross_recipient_third_party_in_shop_context() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let part_owner = UserId::from(2);
        let third_party = UserId::from(5);
        let shop_id = ShopId::from(7);
        let mut shop_rx = core.subscribe_shop(shop_id, acting_user).await;
        let mut third_rx = core
            .install_executor_for_test(third_party, ExecutorStatus::Running)
            .await;
        let summary = summary_with_notification(third_party);

        core.deliver(
            context(acting_user, Some(shop_id), Some(part_owner)),
            summary.clone(),
        )
        .await;

        // The third party's personal stream receives the notification.
        let got_third = third_rx
            .try_recv()
            .expect("the third party's personal stream receives the notification");
        assert_eq!(got_third, summary);
        // The shop stream receives a frame carrying the notification.
        let got_shop = shop_rx
            .try_recv()
            .expect("the shop stream receives the frame");
        assert!(
            got_shop
                .notifications
                .live()
                .iter()
                .any(|n| n.recipient == third_party),
            "the shop stream's frame carries the cross-recipient notification, got {got_shop:?}"
        );
        Ok(())
    }

    /// A shop-context read of a transient notification (a dismiss: tombstone,
    /// no live entry) reaches the acting user's personal stream and fans out
    /// to their open shop streams (the ones other than the session's shop): a
    /// dismiss in the shop tab must clear the private tab and the other shop
    /// tabs (follow-up to issue #687).
    #[tokio::test]
    async fn deliver_shop_context_transient_read_to_personal_stream() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let session_shop = ShopId::from(7);
        let other_shop = ShopId::from(8);
        let mut acting_rx = core
            .install_executor_for_test(acting_user, ExecutorStatus::Running)
            .await;
        let mut other_shop_rx = core.subscribe_shop(other_shop, acting_user).await;
        let mut summary = Summary::default();
        summary
            .notifications
            .tombstone(NotificationId::from(Uuid::now_v7()));

        core.deliver(
            context(acting_user, Some(session_shop), None),
            summary.clone(),
        )
        .await;

        // The acting user's personal stream gets the read frame (the
        // shop-context catch-up).
        let got = acting_rx
            .try_recv()
            .expect("the personal stream gets the read frame");
        assert_eq!(got, summary);
        // The fan-out reaches the acting user's OTHER shop stream (the
        // session's shop is skipped — it already got the frame).
        let got_other = other_shop_rx
            .try_recv()
            .expect("the fan-out reaches the acting user's other shop stream");
        assert_eq!(got_other, summary);
        Ok(())
    }

    /// A shop-context read of a persistent notification (a dismiss: `seen`
    /// upsert) reaches the acting user's personal stream and fans out to their
    /// open shop streams (the ones other than the session's shop).
    #[tokio::test]
    async fn deliver_shop_context_persistent_read_to_personal_stream() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let session_shop = ShopId::from(7);
        let other_shop = ShopId::from(8);
        let mut acting_rx = core
            .install_executor_for_test(acting_user, ExecutorStatus::Running)
            .await;
        let mut other_shop_rx = core.subscribe_shop(other_shop, acting_user).await;
        let summary = summary_with_notification(acting_user);

        core.deliver(
            context(acting_user, Some(session_shop), None),
            summary.clone(),
        )
        .await;

        let got = acting_rx
            .try_recv()
            .expect("the personal stream gets the read frame");
        assert_eq!(got, summary);
        let got_other = other_shop_rx
            .try_recv()
            .expect("the fan-out reaches the acting user's other shop stream");
        assert_eq!(got_other, summary);
        Ok(())
    }

    /// A user-context read (a dismiss in the private tab) fans the frame out
    /// to the acting user's open shop streams: a dismiss in the private tab
    /// must clear the shop tabs (follow-up to issue #687). The personal stream
    /// gets the frame via the normal user-context delivery.
    #[tokio::test]
    async fn deliver_user_context_read_fans_out_to_shop_streams() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let shop_id = ShopId::from(8);
        let mut acting_rx = core
            .install_executor_for_test(acting_user, ExecutorStatus::Running)
            .await;
        let mut shop_rx = core.subscribe_shop(shop_id, acting_user).await;
        let summary = summary_with_notification(acting_user);

        core.deliver(context(acting_user, None, None), summary.clone())
            .await;

        let got = acting_rx
            .try_recv()
            .expect("the personal stream gets the frame");
        assert_eq!(got, summary);
        let got_shop = shop_rx
            .try_recv()
            .expect("the fan-out reaches the acting user's shop stream");
        assert_eq!(got_shop, summary);
        Ok(())
    }

    /// A user-context write whose summary carries a shop (issue #687) fans the
    /// frame out to the notification recipient's open shop streams (follow-up
    /// to issue #687): the recipient's shop views show all their
    /// notifications, like the hydration snapshot — a new notification must
    /// appear live in their other open shop tabs, not only on the next
    /// hydration. The fan-out frame carries only the notifications.
    #[tokio::test]
    async fn deliver_creation_fans_out_to_recipient_shop_streams() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let acting_user = UserId::from(1);
        let shop_owner = UserId::from(2);
        let registered_shop = ShopId::from(7);
        let other_shop = ShopId::from(8);
        let mut shop_owner_stream_rx = core.subscribe_shop(other_shop, shop_owner).await;
        let mut part = part_owned_by(acting_user);
        part.shop = Some(registered_shop);
        let mut summary = Summary::default();
        summary.parts.upsert(part);
        summary.shops.upsert(shop_owned_by(shop_owner));
        let notification = Notification {
            id: NotificationId::from(Uuid::now_v7()),
            recipient: shop_owner,
            lifecycle: NotificationLifecycle::Transient,
            kind: NotificationKind::PartRegistered,
            subject: NotificationSubject::Part(PartId::from(1)),
            message: None,
            state: NotificationState::New,
            created_at: sample_purchase_date(),
        };
        summary.notifications.upsert(notification.clone());
        // The fan-out frame carries only the notification (the shop-scoped
        // snapshot filters parts to the current shop, so a full summary would
        // pollute the recipient's other tabs' parts maps).
        let mut expected = Summary::default();
        expected.notifications.upsert(notification);

        core.deliver(context(acting_user, None, None), summary.clone())
            .await;

        // The fan-out reaches the recipient's other shop stream with a
        // notifications-only frame.
        let got = shop_owner_stream_rx
            .try_recv()
            .expect("the fan-out reaches the recipient's shop stream");
        assert_eq!(got, expected);
        Ok(())
    }

    /// An empty `Summary` pushes nothing, to any stream (spec §4.5).
    #[tokio::test]
    async fn deliver_empty_summary_is_a_noop() -> TbResult<()> {
        let core = Core::new(Duration::from_secs(60));
        let user_id = UserId::from(1);
        let mut acting_rx = core
            .install_executor_for_test(user_id, ExecutorStatus::Running)
            .await;

        core.deliver(context(user_id, None, None), Summary::default())
            .await;

        assert!(
            acting_rx.try_recv().is_err(),
            "an empty summary pushes nothing"
        );
        Ok(())
    }
}
