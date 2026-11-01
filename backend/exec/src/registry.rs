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
//! the service-API facade over the per-user executor's lifecycle core.
//!
//! The registry **owns the [`TxnSource`]** (injected at construction — the
//! only non-axum holder of the seam under the zero-reference bar) and holds
//! the lifecycle core ([`core`]) — the deep "how does an executor stays
//! alive" machinery: the per-user executor map and the per-shop frame-channel
//! map, the spawn-on-first-demand / reap-when-no-stream / respawn logic, the
//! wake signal, the stop-all teardown, the shop-stream lifecycle, and the
//! frame-pusher surface. This module is the thin **facade**: the
//! [`crate::ServiceApi`] impl (the per-user request channel as its ordered
//! surface, spec #558 §3, and the direct ops as its direct surface,
//! spec #558 §5) and the construction API, delegating every lifecycle call to
//! the core. Presentation layers call it through the [`crate::ServiceApi`]
//! trait and never name a seam type.
//!
//! The core is source-agnostic: the facade owns the [`TxnSource`] and the
//! `Box<dyn FramePusher>` the loop routes committed `Summary`s through, and
//! passes both to the core's spawn. The facade's `subscribe_shop` keeps the
//! ownership read (a `Shop` store call, ADR-0010) and delegates only the
//! channel creation to the core. The [`ExecutorStatus`] settled-fate enum
//! lives in the core and is re-exported here so `crate::ExecutorStatus`
//! resolves unchanged.

use std::time::Duration;

use oauth2::RefreshToken;
use tokio::sync::{broadcast, oneshot, watch};

use tb_domain::{
    Activity, ActivityId, ApiRead, ApiWrite, Error, PartId, ReadOutcome, Session, ShopId, Summary,
    TbResult, UserId, WriteOutcome,
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
use crate::message::Request;
use crate::{Txn, TxnSource};

mod core;

pub use core::ExecutorStatus;
use core::{Core, Executor};

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

/// The per-user executor registry (spec §4.4): the service-API facade over
/// the per-user executor's lifecycle core (the [`core`] module). The core
/// holds the live executor per [`UserId`] (shared across requests — the
/// `AppState` is `Clone`, so an `Arc`) and spawns it on the first demand
/// (SSE connect or write), reaped when no stream is attached (the loop
/// decides — an attached stream keeps the executor alive, the idle window is
/// the queue re-probe cadence), respawned on the next demand — the streams
/// re-register on the respawned executor's frame channel.
///
/// Owns the [`TxnSource`] (generic `S`; `DbPool` in production, the seam
/// suite's doubles in the tests) — the seam is injected at construction and
/// every method uses it, so the presentation layer never names it.
#[derive(Clone)]
pub struct Registry<S: TxnSource + Clone + 'static> {
    source: S,
    /// The lifecycle core: the per-user executor map, the per-shop
    /// frame-channel map, and the spawn/reap/wake/stop-all logic. Source-
    /// agnostic — the facade owns the [`TxnSource`] and the `Box<dyn
    /// FramePusher>` it passes to the core's spawn.
    core: Core,
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

    /// Push a frame to every open shop stream `user_id` owns (follow-up to
    /// issue #687): the recipient's shop views show all their
    /// notifications — the shop-scoped hydration snapshot loads them all —
    /// so a notification's frame must reach each of their open shop
    /// streams, not only the shop the operation named. `skip` are the shops
    /// already delivered to (the session's own shop, the summary's shops) —
    /// a second frame on the same channel is noise. A no-op when the user
    /// has no open shop stream (a not-yet-connected client catches up on
    /// hydration, spec §4.5).
    async fn push_to_recipient_shops(&self, user_id: UserId, summary: Summary, skip: &[ShopId]);
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
            core: Core::new(idle_timeout),
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
        self.core.wake(user_id).await
    }

    /// Stop every live executor (the admin rescan's global re-hydration,
    /// issue #446): abort every task (the watchdog records `Dead` on the
    /// abort `JoinError`, like a panic) and clear the executor map — the
    /// core's `stop_all`. The next `subscribe`/`write` respawns fresh.
    pub async fn stop_all(&self) {
        self.core.stop_all().await
    }

    /// A subscription to the user's executor's consumed-Strava-event channel
    /// (spec §6.4): spawning the executor if it is not running. The admin
    /// sync subscribes **before** it enqueues its event, so the completion
    /// signal cannot be missed.
    pub async fn events(&self, user_id: UserId) -> TbResult<broadcast::Receiver<StravaEvent>> {
        let pusher: Box<dyn FramePusher> = Box::new(self.clone());
        self.core.events(&self.source, user_id, pusher).await
    }

    /// Push a frame to the user's SSE streams (test helper: the production
    /// path pushes frames from the loop via `tb_exec::push_frame`).
    pub async fn send_frame(&self, user_id: UserId, summary: Summary) {
        self.core.send_frame(user_id, summary).await
    }

    /// The user's executor's current fate (test helper).
    pub async fn status(&self, user_id: UserId) -> Option<ExecutorStatus> {
        self.core.status(user_id).await
    }

    /// Whether `shop_id`'s stream is open (test helper): the channel exists
    /// and has a receiver (the "open shop" check, spec #517). Dead channels
    /// (no receiver) are reaped on the way, so this is the observable form
    /// of "the channel is dropped when its last receiver disconnects".
    #[cfg(test)]
    pub async fn shop_open(&self, shop_id: ShopId) -> bool {
        self.core.shop_open(shop_id).await
    }

    /// Test helper: install a synthetic executor for `user_id` (bypassing
    /// the core's `ensure_running`, which needs a Strava session the unit
    /// fakes don't have) and return a receiver on its frame channel. The
    /// executor's fate is `status`.
    #[cfg(test)]
    pub async fn install_executor_for_test(
        &self,
        user_id: UserId,
        status: ExecutorStatus,
    ) -> broadcast::Receiver<Summary> {
        self.core.install_executor_for_test(user_id, status).await
    }

    /// Lock the core, ensure the user's executor is live (spawning it if it
    /// is absent or settled), and pull one field from it: the shared shape
    /// of `subscribe`, `events`, and `write`. The facade builds the
    /// `Box<dyn FramePusher>` (from `self.clone()`, which shares the core's
    /// maps) and passes it with the source to the core; the lock is dropped
    /// when this returns, before the caller's next await.
    async fn exec_field<R>(&self, user_id: UserId, f: impl FnOnce(&Executor) -> R) -> TbResult<R> {
        let pusher: Box<dyn FramePusher> = Box::new(self.clone());
        self.core.exec_field(&self.source, user_id, pusher, f).await
    }
}

/// The frame push surface (spec #517, ticket #582): the executor routes a
/// completed `Summary` to a shop stream or a user's personal stream through
/// these. Each delegates to the core, which locks its own map (the shop
/// stream map, the executor map) — the two are independent, so a push never
/// holds both locks.
#[async_trait::async_trait]
impl<S> FramePusher for Registry<S>
where
    S: TxnSource + Clone + 'static,
{
    /// Push to `shop_id`'s stream (spec #517): the core reaps the dead
    /// channels, then sends on the shop channel if it exists.
    async fn push_to_shop(&self, shop_id: ShopId, summary: Summary) {
        self.core.push_to_shop(shop_id, summary).await
    }

    /// Push to `user_id`'s personal stream (spec #517): the core sends on
    /// the user's executor's frame channel if the executor is running; a
    /// no-op otherwise.
    async fn push_to_user(&self, user_id: UserId, summary: Summary) {
        self.core.push_to_user(user_id, summary).await
    }

    /// Push to every open shop stream `user_id` owns (follow-up to issue
    /// #687): the core reaps the dead channels, then sends on each channel
    /// whose owner tag is `user_id`, skipping the shops already delivered
    /// to.
    async fn push_to_recipient_shops(&self, user_id: UserId, summary: Summary, skip: &[ShopId]) {
        self.core
            .push_to_recipient_shops(user_id, summary, skip)
            .await
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
    async fn hydrate_gear(&self, session: Box<dyn Session>) -> TbResult<Summary> {
        let user_id = session.user_id();
        let (request, reply) = Request::new_gear(session);
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
    ///
    /// The ownership read is a `Shop` store call (ADR-0010), so it stays
    /// here in the facade; the channel creation is delegated to the core.
    async fn subscribe_shop(
        &self,
        user_id: UserId,
        shop_id: ShopId,
    ) -> TbResult<broadcast::Receiver<Summary>> {
        // Verify ownership in a transaction (the web layer maps `Forbidden`
        // to a 403).
        let mut store = self.source.begin().await?;
        let shop = shop_id.read(&mut store).await?;
        store.commit().await?;
        if shop.owner != user_id {
            return Err(Error::Forbidden(
                "You must be the shop owner to open this shop's stream".into(),
            ));
        }

        // Create the shop channel on demand and subscribe (the "open shop"
        // check is implicit: `receiver_count() > 0`). The owner tag is the
        // verified caller (follow-up to issue #687): it lets a
        // notification frame fan out to every open shop stream of its
        // recipient.
        Ok(self.core.subscribe_shop(shop_id, user_id).await)
    }

    /// Enqueue the admin [`AllStats`](Request::AllStats) read on the user's
    /// executor and await its stats (spec #558 §3.3): the admin's `GET
    /// /user/all` rides the admin user's own channel — `get_all_stats` is a
    /// `tb_strava` op, so it is an exec-level variant, dispatched loop-side
    /// in the loop's own transaction. The oneshot wait is unbounded, like a
    /// read's (spec #558 §3.7).
    async fn all_stats(&self, session: Box<dyn Session>) -> TbResult<Vec<StravaStat>> {
        let user_id = session.user_id();
        let (request, reply) = Request::new_all_stats(session);
        self.send_request(user_id, request, reply, "stats read")
            .await
    }

    /// The admin rescan (spec #558 §5.2): one transaction
    /// (`Activity::rescan_all`), then stop every live executor — the
    /// rescan is all-users maintenance that bypasses the executors (the
    /// spec §3/§6.4 carve-out), so no frame carries its effect: open
    /// streams end, and each client's native reconnect + catch-up snapshot
    /// re-hydrates it (issue #446).
    async fn rescan(&self, _session: Box<dyn Session>) -> TbResult<()> {
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
    async fn sync(
        &self,
        _session: Box<dyn Session>,
        user_id: UserId,
        event_time: i64,
    ) -> TbResult<()> {
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

    /// Trigger processing for all (or one) users (spec #558 §5.2): admin-only
    /// (the acting `session` must be an admin — `Forbidden` otherwise). One
    /// transaction (`sync_users`) queuing a `Sync` event per (non-disabled)
    /// user, then — per queued user — spawn the executor (a no-op if it is
    /// already running) and wake it. Spawning is what makes the queue drain
    /// for users whose executor is not running (offline): the wake alone is
    /// a signal to a *running* executor only, and the DB queue is the source
    /// of truth. `user_id` narrows the trigger to one user (the route's
    /// `?user_id=` query).
    async fn trigger_processing(
        &self,
        mut session: Box<dyn Session>,
        user_id: Option<UserId>,
        event_time: i64,
        migrate: bool,
    ) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        let queued = sync_users(&mut *session, user_id, event_time, migrate, &mut store).await?;
        store.commit().await?;
        // Per queued user: spawn the executor (a no-op if it is already
        // running) so it drains the queued Sync even when the user is
        // offline, then wake it (interrupt its idle wait so the Sync is
        // processed at once).
        for user_id in queued {
            self.exec_field(user_id, |_| ()).await?;
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
    /// `tb_strava` `strava_url` op (which checks the part's ownership), then
    /// the Strava URL.
    async fn gear_url(&self, mut session: Box<dyn Session>, id: PartId) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let uri = gear::strava_url(i32::from(id), &mut *session, &mut store).await?;
        store.commit().await?;
        Ok(uri)
    }

    /// The activity redirect URL (spec #558 §5.2): one transaction — the
    /// `tb_strava` `strava_url` op (which checks the activity's ownership),
    /// then the Strava URL.
    async fn activity_url(&self, session: Box<dyn Session>, id: ActivityId) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let uri = activity::strava_url(i64::from(id), &*session, &mut store).await?;
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

    /// The deauthorization (spec #558 §5.2): one transaction —
    /// `tb_strava::user_deauthorize` (which verifies the caller is an admin)
    /// and the target `tbid`. No wake, no message; the existing race (deauth
    /// deletes the target's queued events outside the target's loop) is
    /// preserved.
    async fn deauthorize(&self, mut session: Box<dyn Session>, tbid: UserId) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        tb_strava::user_deauthorize(&mut *session, tbid, &mut store).await?;
        store.commit().await?;
        Ok(())
    }

    /// The user delete (spec #558 §5.2): one transaction —
    /// `tb_strava::user_delete` (which verifies the caller is an admin) and
    /// the target `tbid` (the deauthorization plus the row deletions). No
    /// wake, no message; the existing race is preserved.
    async fn delete_user(&self, mut session: Box<dyn Session>, tbid: UserId) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        tb_strava::user_delete(&mut *session, tbid, &mut store).await?;
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
