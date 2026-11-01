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

    /// Reap shop channels whose last receiver has disconnected (the "open
    /// shop" check is implicit: `receiver_count() > 0`). Called on every
    /// shop-map access (`subscribe_shop`, `push_to_shop`, `shop_open`) so a
    /// dead channel does not linger in the map.
    fn reap_shop_streams(map: &mut HashMap<ShopId, ShopStream>) {
        map.retain(|_, stream| stream.sender.receiver_count() > 0);
    }

    /// Lock the core, ensure the user's executor is live (spawning it if it
    /// is absent or settled), and pull one field from it: the shared shape
    /// of `subscribe`, `wake`, `events`, and `write`. The lock is dropped
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
