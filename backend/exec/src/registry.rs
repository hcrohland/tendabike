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

//! The per-user executor registry (ADR-0007, executable spec #558 §6):
//! the live executor per [`UserId`], shared across requests.
//!
//! The registry **owns the [`TxnSource`]** (injected at construction — the
//! only non-axum holder of the seam under the zero-reference bar) and
//! spawns the per-user executor task on demand (the first SSE connect
//! **or** the first API write), reaped when no stream is attached (spec
//! §4.4, amended — #446: an attached stream keeps it alive; the idle
//! window is the queue re-probe cadence). A second connect (or write)
//! finds the running task rather than spawning a duplicate, and a reaped
//! task is respawned on the next demand — the stream re-registers on the
//! respawned executor's frame channel.
//!
//! The registry is the service API's single handle: the per-user request
//! channel is its **ordered surface** (FIFO reads/writes, spec #558 §3),
//! and the direct ops (rescan, sync, login upsert, webhook accept, the
//! redirect URL reads, deauth/delete — spec #558 §5) are its **direct
//! surface**: methods that own their own transaction(s) and any lifecycle
//! control. Presentation layers call it through the [`crate::ServiceApi`]
//! trait and never name a seam type.
//!
//! The executor's fate is shared through a `watch` channel (a `JoinHandle`
//! is not `Clone`, so it cannot be handed to both the registry and the
//! stream tasks): the spawned task records [`ExecutorStatus::Reaped`] on a
//! clean idle exit and [`ExecutorStatus::Dead`] on a lifecycle error or
//! panic. A clone of the frame sender is kept alive in the task until
//! *after* that record is written, so a stream that sees the frame channel
//! close already sees the settled status.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use log::debug;
use oauth2::RefreshToken;
use tokio::sync::{Mutex, Notify, broadcast, oneshot, watch};

use tb_domain::{
    Activity, ActivityId, ApiRead, ApiWrite, Error, PartId, ReadOutcome, Session, ShopId,
    ShopStore, Summary, TbResult, UserId, WriteOutcome,
};
use tb_strava::{
    StravaStat, StravaUser, activity,
    client::StravaAthleteInfo,
    event::{
        Event as StravaEvent, InEvent, insert_sync, is_sync_for_user, sync_queued_for_user,
        sync_users,
    },
    gear,
};

use crate::api::ServiceApi;
use crate::message::{Request, RequestSender};
use crate::{Txn, TxnSource, request_channel, run};

/// The loop's queue re-probe cadence while it is idle with streams attached
/// (spec §4.4, amended — #446): an attached stream keeps the executor alive
/// (it is reaped only when no stream is attached), so the window sets how
/// often the idle loop re-probes the `strava_events` queue. A stream-less
/// executor (a write-only spawn) reaps immediately after its work.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// The SSE heartbeat cadence (spec §4.5): a `: ping` comment keeps the
/// connection alive (and proxies) while the executor is idle (or reaped).
/// The web layer's SSE `KeepAlive` wiring uses this constant.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// The per-user frame channel's capacity: a burst of frames for a lagged
/// stream is dropped (the client's reconnect + full refresh covers it), so a
/// modest bound is enough.
const BROADCAST_CAPACITY: usize = 64;

/// The executor's minimal [`Session`] (spec #602): the user's identity only.
#[derive(Debug, Clone, Copy)]
pub struct ExecutorSession {
    id: UserId,
}

impl ExecutorSession {
    /// Build the executor's session for `id`.
    pub fn new(id: UserId) -> Self {
        Self { id }
    }
}

impl Session for ExecutorSession {
    fn user_id(&self) -> UserId {
        self.id
    }

    fn is_admin(&self) -> bool {
        false
    }

    fn shop(&self) -> Option<ShopId> {
        None
    }
}

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
/// (spec §4.4/§4.6), and the task's abort handle (the registry's `stop_all`
/// tears the executor down — rescan, issue #446). The `AbortHandle` (not
/// the `JoinHandle`) is stored: the watchdog keeps the original handle to
/// `await` and record `Dead` on a panic **or** the abort `JoinError`.
#[derive(Clone)]
struct Executor {
    status: watch::Receiver<ExecutorStatus>,
    /// The request handlers' door into this user's loop (spec #558 §3.2):
    /// enqueue a `Request` here and await its oneshot.
    request_sender: RequestSender,
    frames: broadcast::Sender<Summary>,
    /// Every Strava event the loop consumed (spec §6.4): the admin sync's
    /// completion signal, correlated by the event's identity.
    events: broadcast::Sender<StravaEvent>,
    /// The in-memory wake signal (spec §4.4/§4.6): fired by `Registry::wake`
    /// after a Strava event is queued; the running loop selects on it, so
    /// the event is processed at once, even mid-idle-sleep.
    wake: Arc<Notify>,
    /// Aborts the executor task (`stop_all`): the watchdog records `Dead`
    /// on the abort `JoinError`, like a panic.
    task: tokio::task::AbortHandle,
}

/// The per-user executor registry (spec §4.4): the live executor per
/// [`UserId`], shared across requests (the `AppState` is `Clone`, so an
/// `Arc`). Spawn on the first demand (SSE connect or write), reap when no
/// stream is attached (the loop decides — an attached stream keeps the
/// executor alive, the idle window is the queue re-probe cadence), respawn
/// on the next demand — the streams re-register on the respawned executor's
/// frame channel.
///
/// Owns the [`TxnSource`] (generic `S`; `DbPool` in production, the seam
/// suite's doubles in the tests) — the seam is injected at construction and
/// every method uses it, so the presentation layer never names it.
#[derive(Clone)]
pub struct Registry<S: TxnSource + Clone + 'static> {
    source: S,
    executors: Arc<Mutex<HashMap<UserId, Executor>>>,
    /// The per-shop frame channels (spec #517, ticket #582): a standalone
    /// `broadcast` channel per shop, created on demand by `subscribe_shop`
    /// and reaped when its last receiver disconnects (the "open shop" check
    /// is implicit: `receiver_count() > 0`). A separate mutex from the
    /// executor map keeps the two lock scopes tight (an ownership read never
    /// holds the shop-map lock, and a shop push never holds the executor
    /// lock).
    shop_streams: Arc<Mutex<HashMap<ShopId, broadcast::Sender<Summary>>>>,
    idle_timeout: Duration,
}

/// The frame push surface the executor uses to route a completed `Summary`
/// to an arbitrary target (spec #517, ticket #582): a shop stream
/// (`push_to_shop`) or a user's personal stream (`push_to_user`). The
/// [`Registry`] implements it; the executor calls it in a shop context,
/// after a write commit (ticket #584 wires the context-based routing).
///
/// Both methods are infallible: a push to a target with no live receiver
/// (a reaped shop channel, a settled executor) is a no-op — a lost frame
/// just means a not-yet-connected or gone client, and the client's
/// reconnect + catch-up snapshot covers it (spec §4.5).
#[async_trait::async_trait]
pub trait FramePusher: Send + Sync {
    /// Push a frame to `shop_id`'s stream: sends on the shop channel if it
    /// exists and has a receiver (the "open shop" check is implicit:
    /// `receiver_count() > 0`); a no-op otherwise.
    async fn push_to_shop(&self, shop_id: ShopId, summary: Summary);

    /// Push a frame to `user_id`'s personal stream: sends on the user's
    /// executor's frame channel if the executor is running; a no-op
    /// otherwise (no executor, or a settled one).
    async fn push_to_user(&self, user_id: UserId, summary: Summary);
}

impl<S> Registry<S>
where
    S: TxnSource + Clone + 'static,
{
    /// The registry with an explicit idle cadence (test ergonomics: a
    /// short idle cadence makes the executor's lifecycle observable).
    /// The SSE heartbeat cadence is the crate constant
    /// [`HEARTBEAT_INTERVAL`] — the `KeepAlive` wiring uses it directly.
    pub fn new(source: S, idle_timeout: Duration) -> Self {
        Self {
            source,
            executors: Arc::new(Mutex::new(HashMap::new())),
            shop_streams: Arc::new(Mutex::new(HashMap::new())),
            idle_timeout,
        }
    }

    /// The registry with the production default (the 60s idle re-probe
    /// cadence).
    pub fn with_defaults(source: S) -> Self {
        Self::new(source, IDLE_TIMEOUT)
    }

    /// Fire the wake signal for the user's executor (spec §4.4/§4.6): a
    /// pure signal to a **running** executor — it interrupts the loop's
    /// idle wait so a just-queued Strava event is processed at once. It
    /// never spawns an executor: a reaped (or not-yet-spawned) executor
    /// picks the event up on its next spawn (an SSE connect or an API
    /// write) — the DB queue is the source of truth. Infallible by design:
    /// a missed wake only delays the event to the next spawn, so the
    /// webhook treats it as non-critical.
    pub async fn wake(&self, user_id: UserId) {
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
    /// clearing the map drops the registry's, so both channels close and
    /// open streams see the close and end (their clients reconnect and the
    /// catch-up snapshot re-hydrates them; an in-flight admin sync hits its
    /// already-handled queue-fallback path) — then clear the map. The next
    /// `subscribe`/`write` respawns fresh (`ensure_running`'s existing
    /// behavior). The lock is held across the loop, so no executor can be
    /// spawned between the aborts and the clear.
    pub async fn stop_all(&self) {
        let mut map = self.executors.lock().await;
        for exec in map.values() {
            exec.task.abort();
        }
        map.clear();
    }

    /// A subscription to the user's executor's consumed-Strava-event channel
    /// (spec §6.4): spawning the executor if it is not running. The admin
    /// sync subscribes **before** it enqueues its event, so the completion
    /// signal cannot be missed. `session` is the `Box<dyn Session>`
    /// threaded into the loop on spawn.
    pub async fn events(&self, user_id: UserId) -> TbResult<broadcast::Receiver<StravaEvent>> {
        self.exec_field(user_id, |exec| exec.events.subscribe())
            .await
    }

    /// Push a frame to the user's SSE streams (test helper: the production
    /// path pushes frames from the loop via `tb_exec::push_frame`).
    pub async fn send_frame(&self, user_id: UserId, summary: Summary) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id) {
            let _ = exec.frames.send(summary);
        }
    }

    /// The user's executor's current fate (test helper).
    pub async fn status(&self, user_id: UserId) -> Option<ExecutorStatus> {
        let map = self.executors.lock().await;
        map.get(&user_id).map(|exec| *exec.status.borrow())
    }

    /// Whether `shop_id`'s stream is open (test helper): the channel exists
    /// and has a receiver (the "open shop" check, spec #517). Dead channels
    /// (no receiver) are reaped on the way, so this is the observable form
    /// of "the channel is dropped when its last receiver disconnects".
    #[cfg(test)]
    pub async fn shop_open(&self, shop_id: ShopId) -> bool {
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
    pub async fn install_executor_for_test(
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

    /// Reap shop channels whose last receiver has disconnected (the "open
    /// shop" check is implicit: `receiver_count() > 0`). Called on every
    /// shop-map access (`subscribe_shop`, `push_to_shop`, `shop_open`) so a
    /// dead channel does not linger in the map.
    fn reap_shop_streams(map: &mut HashMap<ShopId, broadcast::Sender<Summary>>) {
        map.retain(|_, tx| tx.receiver_count() > 0);
    }

    /// Lock the registry, ensure the user's executor is live (spawning it if
    /// it is absent or settled), and pull one field from it: the shared shape
    /// of `subscribe`, `wake`, `events`, and `write`. The lock is dropped
    /// when this returns, before the caller's next await.
    async fn exec_field<R>(&self, user_id: UserId, f: impl FnOnce(&Executor) -> R) -> TbResult<R> {
        let mut map = self.executors.lock().await;
        // The executor routes a committed `Summary` by context (spec #517,
        // ticket #584) through the registry itself: `self.clone()` shares the
        // shop-stream and executor maps (the `Arc`s), so its `push_to_shop` /
        // `push_to_user` reach the live shop streams and executors.
        let pusher: Box<dyn FramePusher> = Box::new(self.clone());
        let exec = Self::ensure_running(&mut map, &self.source, user_id, self.idle_timeout, pusher)
            .await?;
        Ok(f(&exec))
    }

    /// Return the user's live executor, spawning it if it is absent or has
    /// settled (reaped/dead): build the loop's identity-only session (the
    /// shop-scoping fix — the per-request shop rides on each `Request`, and
    /// the queue probe / Strava events / gear lanes read the token from the
    /// `TokenCache`, so the spawn-time session's shop is irrelevant), open a
    /// fresh request channel and frame channel, and spawn the `tb_exec` loop.
    async fn ensure_running(
        map: &mut HashMap<UserId, Executor>,
        source: &S,
        user_id: UserId,
        idle_timeout: Duration,
        pusher: Box<dyn FramePusher>,
    ) -> TbResult<Executor> {
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
        // The loop's session is identity-only (the shop-scoping fix): the
        // per-request shop rides on each `Request`, and the queue probe /
        // Strava events / gear lanes read the token from the `TokenCache`.
        let session: Box<dyn Session> = Box::new(ExecutorSession::new(user_id));
        let join = tokio::spawn(async move {
            let result = run(
                owned_source,
                session,
                reader,
                frames_for_run,
                pusher,
                events_for_run,
                idle_timeout,
                wake_for_run,
            )
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

/// The frame push surface (spec #517, ticket #582): the executor routes a
/// completed `Summary` to a shop stream or a user's personal stream through
/// these. Each locks its own map (the shop stream map, the executor map) —
/// the two are independent, so a push never holds both locks.
#[async_trait::async_trait]
impl<S> FramePusher for Registry<S>
where
    S: TxnSource + Clone + 'static,
{
    /// Push to `shop_id`'s stream (spec #517): reap the dead channels, then
    /// send on the shop channel if it exists (after the reap, an existing
    /// entry has a receiver, so the send is not dropped).
    async fn push_to_shop(&self, shop_id: ShopId, summary: Summary) {
        let mut map = self.shop_streams.lock().await;
        Self::reap_shop_streams(&mut map);
        if let Some(tx) = map.get(&shop_id) {
            let _ = tx.send(summary);
        }
    }

    /// Push to `user_id`'s personal stream (spec #517): send on the user's
    /// executor's frame channel if the executor is running; a no-op
    /// otherwise (no executor, or a settled one).
    async fn push_to_user(&self, user_id: UserId, summary: Summary) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            let _ = exec.frames.send(summary);
        }
    }
}

/// The service API's impl for the registry (spec #558 §2): the per-user
/// channel ops ride [`Self::send_request`] (spawn-or-join the executor,
/// enqueue, await the oneshot); the direct ops own their own
/// transaction(s) and any lifecycle control.
#[async_trait::async_trait]
impl<S> ServiceApi for Registry<S>
where
    S: TxnSource + Clone + 'static,
{
    /// Enqueue one [`ApiRead`] on the user's executor and await its data
    /// (spec #558 §3): the loop runs the read in its own transaction
    /// (read-only within it by spec) and resolves the [`ReadOutcome`] —
    /// the route maps a `Err` through the existing `AppError` helper,
    /// exactly as today.
    async fn read(&self, session: Box<dyn Session>, read: ApiRead) -> TbResult<ReadOutcome> {
        let user_id = session.user_id();
        // The acting session rides on the request (the shop-scoping fix): the
        // loop runs the read against it. The executor's spawn-time session is
        // the exception — used only for the background Strava lanes (the queue
        // probe + events), which need just the fixed user_id + the TokenCache.
        let (request, reply) = Request::new_read(session, read);
        self.send_request(user_id, request, reply, "read").await
    }

    /// Enqueue one [`ApiWrite`] on the user's executor and await the full
    /// outcome (the `Summary` plus, for the descend, its match report)
    /// (spec §6.2): the oneshot is unbounded — through a 15-minute
    /// rate-limit backoff the write may wait long, and that is the spec'd
    /// behavior. The route projects its payload: a create's `201` entity
    /// or the descend's report body from the `WriteOutcome`, a `204`
    /// otherwise.
    async fn write(&self, session: Box<dyn Session>, write: ApiWrite) -> TbResult<WriteOutcome> {
        let user_id = session.user_id();
        // The acting session rides on the request (the shop-scoping fix): the
        // loop runs the write against it. The executor's spawn-time session is
        // the exception — used only for the background Strava lanes (the queue
        // probe + events), which need just the fixed user_id + the TokenCache.
        let (request, reply) = Request::new_write(session, write);
        self.send_request(user_id, request, reply, "write").await
    }

    /// Enqueue the [`Gear`](Request::Gear) hydration write on the user's
    /// executor and await its frame (spec #558 §4): the loop runs
    /// `update_gear` in its own transaction with its own session (a
    /// `tb_strava` op — it cannot be an `ApiWrite` variant) and resolves
    /// with the created parts' `Summary` — the frame (the union of the
    /// parts it created, one entry per part, empty when nothing is new).
    /// The `/user/summary` route runs this **before** its unified `User`
    /// read, FIFO-ordered, so the snapshot body carries the new parts
    /// exactly as today.
    async fn hydrate_gear(&self, user_id: UserId) -> TbResult<Summary> {
        // The hydration runs against an identity-only acting session (the
        // user's own gear — no shop); the request carries it like any other.
        let (request, reply) = Request::new_gear(Box::new(ExecutorSession::new(user_id)));
        self.send_request(user_id, request, reply, "hydration")
            .await
    }

    /// Register a stream for the user (spawning the executor if it is not
    /// running) and return a subscription to its frame channel plus a handle
    /// to its fate, so the stream can tell a clean reap from a dead executor.
    async fn subscribe(
        &self,
        session: Box<dyn Session>,
    ) -> TbResult<(
        broadcast::Receiver<Summary>,
        watch::Receiver<ExecutorStatus>,
    )> {
        let user_id = session.user_id();
        self.exec_field(user_id, |exec| {
            (exec.frames.subscribe(), exec.status.clone())
        })
        .await
    }

    /// Register a shop stream for `shop_id` (spec #517, ticket #582): verify
    /// the caller is the shop owner in a transaction (a non-owner is
    /// `Forbidden`, mapped to a 403 by the web layer), then create the shop
    /// channel on demand and return a subscription to it. No `ExecutorStatus`
    /// — the shop stream is a standalone channel, not tied to the executor's
    /// lifecycle.
    async fn subscribe_shop(
        &self,
        user_id: UserId,
        shop_id: ShopId,
    ) -> TbResult<broadcast::Receiver<Summary>> {
        // Verify ownership in a transaction (the web layer maps `Forbidden`
        // to a 403).
        let mut store = self.source.begin().await?;
        let shop = store.shop_get(shop_id).await?;
        store.commit().await?;
        if shop.owner != user_id {
            return Err(Error::Forbidden(
                "You must be the shop owner to open this shop's stream".into(),
            ));
        }

        // Create the shop channel on demand and subscribe (the "open shop"
        // check is implicit: `receiver_count() > 0`).
        let mut map = self.shop_streams.lock().await;
        Self::reap_shop_streams(&mut map);
        let tx = map
            .entry(shop_id)
            .or_insert_with(|| broadcast::channel(BROADCAST_CAPACITY).0)
            .clone();
        Ok(tx.subscribe())
    }

    /// Enqueue the admin [`AllStats`] read on the user's executor and await
    /// its stats (spec #558 §3.3): the admin's `GET /user/all` rides the
    /// admin user's own channel — `get_all_stats` is a `tb_strava` op, so
    /// it is an exec-level variant, dispatched loop-side in the loop's own
    /// transaction. The oneshot wait is unbounded, like a read's (spec
    /// #558 §3.7).
    async fn all_stats(&self, user_id: UserId) -> TbResult<Vec<StravaStat>> {
        // The stats read runs against an identity-only acting session (it
        // pushes no frame); the request carries it like any other.
        let (request, reply) = Request::new_all_stats(Box::new(ExecutorSession::new(user_id)));
        self.send_request(user_id, request, reply, "stats read")
            .await
    }

    /// The admin rescan (spec #558 §5.2): one transaction
    /// (`Activity::rescan_all`), then stop every live executor — the
    /// rescan is all-users maintenance that bypasses the executors (the
    /// spec §3/§6.4 carve-out), so no frame carries its effect: open
    /// streams end, and each client's native reconnect + catch-up snapshot
    /// re-hydrates it (issue #446).
    async fn rescan(&self) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        Activity::rescan_all(&mut store).await?;
        store.commit().await?;
        self.stop_all().await;
        Ok(())
    }

    /// Run one admin sync for `user_id` (spec #558 §5.2): subscribe to
    /// the executor's consumed-event channel (spawning it if it is not
    /// running), then the enqueue transaction — the target's Strava id
    /// (the queue is keyed by it) **and** the `Sync` event, committed
    /// together — then wake the executor and await that event's
    /// completion (unbounded — the spec'd behavior for the admin
    /// endpoint). The awaited work runs in the target user's loop
    /// transaction.
    ///
    /// The completion is correlated by the event's identity (owner, type,
    /// `event_time`). If the channel closes or lags (the executor settled
    /// mid-wait), the `strava_events` queue — the source of truth (spec
    /// §4.6) — is consulted: a sync no longer queued has been consumed.
    async fn sync(&self, user_id: UserId, event_time: i64) -> TbResult<()> {
        // Subscribe before enqueuing, so the completion cannot be missed
        // (the executor spawns identity-only — the shop-scoping fix).
        let mut done = self.events(user_id).await?;

        // The enqueue txn (spec #558 §5.2): the `Sync` event, committed —
        // the `strava_id` (the queue's key) is resolved inside `tb_strava`.
        let mut store = self.source.begin().await?;
        insert_sync(user_id, event_time, false, &mut store).await?;
        store.commit().await?;

        // Wake the executor (non-critical: the queue is the source of
        // truth). The `events` subscription above already ensured the
        // executor is running, so this only interrupts its idle wait.
        self.wake(user_id).await;

        // Await our sync's completion (unbounded, spec §6.4).
        loop {
            match done.recv().await {
                Ok(event) => {
                    // The correlation is a `tb_strava` concern (the queue is
                    // keyed by the `strava_id`, resolved inside the crate).
                    let mut store = self.source.begin().await?;
                    let is_our_sync =
                        is_sync_for_user(&event, user_id, event_time, &mut store).await?;
                    store.commit().await?;
                    if is_our_sync {
                        return Ok(());
                    }
                    // Some other message completed; keep waiting for ours.
                }
                Err(_) => {
                    // The channel closed (or lagged): the executor settled
                    // without announcing our event. Consult the queue — the
                    // source of truth (spec §4.6): a sync no longer queued
                    // has been consumed; otherwise re-subscribe (respawning
                    // the executor) and keep waiting.
                    let mut store = self.source.begin().await?;
                    let queued = sync_queued_for_user(user_id, event_time, &mut store).await?;
                    store.commit().await?;
                    if !queued {
                        return Ok(());
                    }
                    done = self.events(user_id).await?;
                }
            }
        }
    }

    /// The all-users admin sync (spec #558 §5.2): one transaction
    /// (`sync_users`) queuing a `Sync` event per (non-disabled) user, then
    /// a wake **per queued user** (a running executor drains its queued
    /// sync immediately, matching the per-user variant; a reaped,
    /// or not-yet-spawned, executor is never woken into existence: the
    /// wake is a signal only, and the DB queue is the source of truth).
    /// `user_id` narrows the
    /// sync to one user (the route's `?user_id=` query), as before the
    /// move.
    async fn sync_all(
        &self,
        user_id: Option<UserId>,
        event_time: i64,
        migrate: bool,
    ) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        let queued = sync_users(user_id, event_time, migrate, &mut store).await?;
        store.commit().await?;
        // The per-user wake (spec #558 §5.2): a pure signal to a running
        // executor — it never spawns one, and the DB queue is the source
        // of truth, so a missed wake only delays the sync to the next
        // spawn.
        for user_id in queued {
            self.wake(user_id).await;
        }
        Ok(())
    }

    /// The login upsert (spec #558 §5.2): one transaction
    /// (`StravaUser::upsert` + `is_admin`), returning the identity and the
    /// admin flag. The web layer keeps the query-parse, the scope check,
    /// the HMAC path, the outbound exchange, and the tower-session write —
    /// the wire-shape parsing stays beside the exchange it feeds.
    async fn login_from_token(
        &self,
        athlete: StravaAthleteInfo,
        refresh: Option<RefreshToken>,
    ) -> TbResult<(UserId, bool)> {
        let StravaAthleteInfo {
            id,
            firstname,
            lastname,
            avatar,
            ..
        } = athlete;
        let mut store = self.source.begin().await?;
        let user = StravaUser::upsert(
            id,
            &firstname,
            &lastname,
            &avatar,
            refresh.as_ref(),
            &mut store,
        )
        .await?;
        let is_admin = user.user_id().is_admin(&mut store).await?;
        store.commit().await?;
        Ok((user.user_id(), is_admin))
    }

    /// The webhook accept (spec #558 §5.2): one transaction
    /// (`InEvent::accept` — the queue insert, or the athlete-deauth
    /// branch), then wake the user's executor if an event was queued
    /// (fire-and-forget, `200`). A failed ingest 500s to Strava (its own
    /// retry policy applies).
    async fn accept_event(&self, event: InEvent) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        let user_id = event.accept(&mut store).await?;
        store.commit().await?;
        // Fire the in-memory wake signal (spec §4.4): a pure signal to a
        // running executor — it never spawns one, and the DB queue is the
        // source of truth, so a missed wake only delays the event to the
        // next spawn.
        if let Some(user_id) = user_id {
            self.wake(user_id).await;
        }
        Ok(())
    }

    /// The gear redirect URL (spec #558 §5.2): one transaction — the
    /// executor's [`ExecutorSession`] built in-txn (the user-gated
    /// `tb_strava` `strava_url` op, which checks the part's ownership),
    /// then the Strava URL.
    async fn gear_url(&self, user: UserId, id: PartId) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let mut session = ExecutorSession::new(user);
        let uri = gear::strava_url(i32::from(id), &mut session, &mut store).await?;
        store.commit().await?;
        Ok(uri)
    }

    /// The activity redirect URL (spec #558 §5.2): one transaction — the
    /// executor's [`ExecutorSession`] built in-txn (the user-gated
    /// `tb_strava` `strava_url` op, which checks the activity's ownership),
    /// then the Strava URL.
    async fn activity_url(&self, user: UserId, id: ActivityId) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let session = ExecutorSession::new(user);
        let uri = activity::strava_url(i64::from(id), &session, &mut store).await?;
        store.commit().await?;
        Ok(uri)
    }

    /// The user redirect URL (spec #558 §5.2): one transaction — the
    /// `tb_strava` `strava_url` op for the target's tbid (no session: the
    /// route has no user context to gate on).
    async fn user_url(&self, id: UserId) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let uri = tb_strava::strava_url(id, &mut store).await?;
        store.commit().await?;
        Ok(uri)
    }

    /// The deauthorization (spec #558 §5.2): one transaction — the
    /// target's [`ExecutorSession`] built in-txn, then
    /// `tb_strava::user_deauthorize`. No wake, no message; the existing
    /// race (deauth deletes the target's queued events outside the
    /// target's loop) is preserved.
    async fn deauthorize(&self, tbid: UserId) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        let mut session = ExecutorSession::new(tbid);
        tb_strava::user_deauthorize(&mut session, &mut store).await?;
        store.commit().await?;
        Ok(())
    }

    /// The user delete (spec #558 §5.2): one transaction — the target's
    /// [`ExecutorSession`] built in-txn, then `tb_strava::user_delete`
    /// (the deauthorization plus the row deletions). No wake, no message;
    /// the existing race is preserved.
    async fn delete_user(&self, tbid: UserId) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        let mut session = ExecutorSession::new(tbid);
        tb_strava::user_delete(&mut session, &mut store).await?;
        store.commit().await?;
        Ok(())
    }
}

/// Enqueue a [`Request`] on the user's executor and await its oneshot
/// — the shared shape of the registry's `read`, `hydrate_gear`,
/// `all_stats`, and `write` (spec #558 §3): spawn-or-join the executor,
/// send the request on the user's request channel, and await the oneshot
/// (unbounded — the message is serialized behind any in-flight one,
/// spec #558 §3.7). `dropped` names the message in the reply-drop
/// error ("read", "hydration", "stats read", "write"). `exec_field`
/// drops the registry lock on return, so the unbounded reply await
/// runs without it.
impl<S> Registry<S>
where
    S: TxnSource + Clone + 'static,
{
    async fn send_request<Out>(
        &self,
        user_id: UserId,
        request: Request,
        reply: oneshot::Receiver<TbResult<Out>>,
        dropped: &str,
    ) -> TbResult<Out> {
        let request_sender = self
            .exec_field(user_id, |exec| exec.request_sender.clone())
            .await?;
        if request_sender.send(request).is_err() {
            // The executor died between the liveness check and the send
            // (spec §4.6): the message is lost, the client retries.
            return Err(Error::AnyFailure(anyhow::anyhow!(
                "the executor for user {user_id} is gone"
            )));
        }
        reply.await.map_err(|_| {
            Error::AnyFailure(anyhow::anyhow!(
                "the executor for user {user_id} dropped the {dropped}"
            ))
        })?
    }
}
