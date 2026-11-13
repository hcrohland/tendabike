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
//! The registry **owns the [`TxnSource`]** (injected at construction; the
//! loop's `source` is a clone of it, and the web layer names the
//! [`crate::ServiceApi`] trait, never a seam type) and holds
//! the lifecycle core ([`core`]) — the deep "how does an executor stay
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
//! ownership check (a `ShopId` store call, ADR-0010) and delegates only the
//! channel creation to the core. The [`ExecutorStatus`] settled-fate enum
//! lives in the core and is re-exported here so `crate::ExecutorStatus`
//! resolves unchanged.

use std::time::Duration;

use tokio::sync::{broadcast, oneshot, watch};

use tb_domain::{
    Activity, ActivityId, ApiRead, ApiWrite, Error, PartId, ReadOutcome, Session, ShopId, Store,
    Summary, TbResult, UserId, UserStat, WriteOutcome,
};
use tb_provider::Provider;
use tb_provider::ProviderLane;
use tb_provider::strava::{StravaStore, event::sync_users};

use crate::api::ServiceApi;
use crate::message::Request;
use crate::{DomainFacade, Txn, TxnSource};

mod core;

use core::{Core, Executor};
pub use core::{DeliveryContext, ExecutorStatus};

/// The loop's queue re-probe cadence while it is idle with streams attached
/// (spec §4.4, amended — #446): an attached stream keeps the executor alive
/// (it is reaped only when no stream is attached), so the window sets how
/// often the idle loop re-probes the `strava_events` queue. A stream-less
/// executor (a write-only spawn) reaps immediately after its work.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// The SSE heartbeat cadence (spec §4.5): a `: ping` comment keeps the
/// connection alive (and proxies) while the executor is idle (or reaped).
/// The web layer's SSE `KeepAlive` wiring uses this constant.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

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
pub struct Registry<S, P, D>
where
    S: TxnSource + Clone + 'static,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
{
    source: S,
    /// The domain facade: the executor's door into the domain (spec #795).
    /// Owned by the registry and handed to the core's spawn (a clone per
    /// executor).
    facade: D,
    /// The provider lane: the narrow seam the loop calls through to probe
    /// and process the provider's event queue. The registry owns it and
    /// passes a clone to the core's spawn.
    lane: P,
    /// The lifecycle core: the per-user executor map, the per-shop
    /// frame-channel map, and the spawn/reap/wake/stop-all logic. Source-
    /// agnostic — the facade owns the [`TxnSource`] and the `Box<dyn
    /// FramePusher>` it passes to the core's spawn.
    core: Core<P>,
}

/// The frame push surface the executor loop uses to route a completed
/// `Summary` to its delivery targets (spec #517; collapsed from
/// three push methods to one `deliver`, spec #706): the implementation (the
/// [`Registry`]'s lifecycle core) owns the complete routing policy — given
/// the acting session's context, it inspects the `Summary`'s content and
/// pushes to each target it names.
///
/// The method is infallible: a push to a target with no live receiver
/// (a reaped shop channel, a settled executor) is a no-op — a lost frame
/// just means a not-yet-connected or gone client, and the client's
/// reconnect + catch-up snapshot covers it (spec §4.5).
#[async_trait::async_trait]
pub trait FramePusher: Send + Sync {
    /// Route a [`Summary`] to its delivery targets given the acting
    /// session's context.
    async fn deliver(&self, context: DeliveryContext, summary: Summary);
}

impl<S, P, D> Registry<S, P, D>
where
    S: TxnSource + Clone + 'static,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
{
    /// The registry with an explicit idle cadence (test ergonomics: a
    /// short idle cadence makes the executor's lifecycle observable).
    /// The SSE heartbeat cadence is the crate constant
    /// [`HEARTBEAT_INTERVAL`] — the `KeepAlive` wiring uses it directly.
    pub fn new(source: S, facade: D, lane: P, idle_timeout: Duration) -> Self {
        Self {
            source,
            facade,
            lane,
            core: Core::new(idle_timeout),
        }
    }

    /// The registry with the production default (the 60s idle re-probe
    /// cadence).
    pub fn with_defaults(source: S, facade: D, lane: P) -> Self {
        Self::new(source, facade, lane, IDLE_TIMEOUT)
    }

    /// Fire the wake signal for the user's executor (spec §4.4/§4.6): a
    /// pure signal to a **running** executor — it interrupts the loop's
    /// idle wait so a just-queued Strava event is processed at once. It
    /// never spawns an executor: a reaped (or not-yet-spawned) executor
    /// picks the event up on its next spawn (an SSE connect or an API
    /// write) — the DB queue is the source of truth. Infallible by design:
    /// a missed wake only delays the event to the next spawn, so the
    /// webhook treats it as non-critical.
    pub(crate) async fn wake(&self, user_id: UserId) {
        self.core.wake(user_id).await
    }

    /// Stop every live executor (the admin rescan's global re-hydration):
    /// abort every task (the watchdog records `Dead` on the
    /// abort `JoinError`, like a panic) and clear the executor map — the
    /// core's `stop_all`. The next `subscribe`/`write` respawns fresh.
    pub(crate) async fn stop_all(&self) {
        self.core.stop_all().await
    }

    /// Push a frame to the user's SSE streams (test helper: the production
    /// path delivers frames from the loop through the pusher).
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
    async fn exec_field<R>(
        &self,
        user_id: UserId,
        f: impl FnOnce(&Executor<P>) -> R,
    ) -> TbResult<R> {
        let pusher: Box<dyn FramePusher> = Box::new(self.clone());
        self.core
            .exec_field(&self.source, &self.facade, &self.lane, user_id, pusher, f)
            .await
    }
}

/// The shop-stream subscription (spec #517): the inherent
/// surface with the narrow `ShopStore + Txn` bound (the ownership read
/// needs only the shop sub-trait). The [`ServiceApi`] impl delegates
/// to this; the web layer calls it through the trait.
impl<S, P, D> Registry<S, P, D>
where
    S: TxnSource + Clone + 'static,
    S::Conn: tb_domain::ShopStore + Txn,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
{
    /// Register a shop stream for `shop_id` (spec #517): verify
    /// the caller is the shop owner in a transaction (a non-owner is
    /// `Forbidden`, mapped to a 403 by the web layer), then create the shop
    /// channel on demand and return a subscription to it.
    pub async fn subscribe_shop(
        &self,
        user_id: UserId,
        shop_id: ShopId,
    ) -> TbResult<broadcast::Receiver<Summary>> {
        // Verify ownership in a transaction (the web layer maps `Forbidden`
        // to a 403).
        let mut store = self.source.begin().await?;
        shop_id.check_owner(user_id, &mut store).await?;
        store.commit().await?;

        // Create the shop channel on demand and subscribe (the "open shop"
        // check is implicit: `receiver_count() > 0`). The owner tag is the
        // verified caller: it lets a
        // notification frame fan out to every open shop stream of its
        // recipient.
        Ok(self.core.subscribe_shop(shop_id, user_id).await)
    }
}

/// The frame push surface (spec #517): the executor loop
/// routes a completed `Summary` through this, and the implementation
/// delegates the routing to the core — the deep "where does a frame go?"
/// policy lives there (spec #705/#706), locking its own maps (the shop
/// stream map, the executor map) — the two are independent, so a push never
/// holds both locks.
#[async_trait::async_trait]
impl<S, P, D> FramePusher for Registry<S, P, D>
where
    S: TxnSource + Clone + 'static,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
{
    /// Delegate the routing to the core (the complete frame-routing policy).
    async fn deliver(&self, context: DeliveryContext, summary: Summary) {
        self.core.deliver(context, summary).await
    }
}

/// The service API's impl for the registry (spec #558 §2): the per-user
/// channel ops ride [`Self::send_request`] (spawn-or-join the executor,
/// enqueue, await the oneshot).
#[async_trait::async_trait]
impl<S, P, D> ServiceApi for Registry<S, P, D>
where
    // The generic channel ops (`read`/`write`/`hydrate_gear`/`subscribe`/
    // `all_stats`) ride the executor and need only `Txn` (their domain work
    // rides the facade and the lane). The three provider-neutral direct ops
    // (`rescan`, `subscribe_shop`, `trigger_processing`, ADR-0014) own their
    // own transactions and do their work directly — the rescan's
    // `Activity::rescan_all` (a full `Store`), the shop-ownership read
    // (`ShopStore + Txn`, carried by `Store`), and the `sync_users` queue op
    // (`StravaStore`) — so this impl carries the full `Store + StravaStore +
    // Txn` bound.
    S: TxnSource + Clone + 'static,
    S::Conn: Store + StravaStore + Txn,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
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
    /// `tb_provider::strava` op — it cannot be an `ApiWrite` variant) and resolves
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

    /// Enqueue the admin [`AllStats`](Request::AllStats) read on the user's
    /// executor and await its stats (spec #558 §3.3): the admin's `GET
    /// /user/all` rides the admin user's own channel — `get_all_stats` is a
    /// `tb_provider::strava` op, so it is an exec-level variant, dispatched loop-side
    /// in the loop's own transaction. The oneshot wait is unbounded, like a
    /// read's (spec #558 §3.7).
    async fn all_stats(&self, session: Box<dyn Session>) -> TbResult<Vec<UserStat>> {
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
    /// re-hydrates it.
    async fn rescan(&self, _session: Box<dyn Session>) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        Activity::rescan_all(&mut store).await?;
        store.commit().await?;
        self.stop_all().await;
        Ok(())
    }

    /// Register a shop stream for `shop_id` (spec #517): verify
    /// the caller is the shop owner in a transaction (a non-owner is
    /// `Forbidden`, mapped to a 403 by the web layer), then create the shop
    /// channel on demand and return a subscription to it. The ownership check
    /// is a `ShopId` store call (ADR-0010); the channel creation is delegated
    /// to the core.
    async fn subscribe_shop(
        &self,
        user_id: UserId,
        shop_id: ShopId,
    ) -> TbResult<broadcast::Receiver<Summary>> {
        self.subscribe_shop(user_id, shop_id).await
    }

    /// Trigger processing for all (or one) users (spec #558 §5.2): admin-only
    /// (the acting `session` must be an admin — `Forbidden` otherwise). One
    /// transaction (`sync_users`) queuing a `Sync` event per (non-disabled)
    /// user, then — per queued user — spawn the executor (a no-op if it is
    /// already running) and wake it. `user_id` narrows the trigger to one
    /// user (the route's `?user_id=` query).
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
}

/// The provider-agnostic provider service's impl for the registry (ADR-0014):
/// the eight direct ops are a thin forward — each opens the transaction, hands
/// the connection (and the concrete entity-op facade) to the provider
/// layer's dispatch free function (the single `match provider`), and commits.
/// No provider dispatch and no driver naming in this crate. The registry owns
/// the store (the transaction boundary, ADR-0014); the dispatch bounds on the
/// two providers' local data stores and takes the facade as a parameter.
#[async_trait::async_trait]
impl<S, P, D> crate::providerservice::ProviderService for Registry<S, P, D>
where
    S: TxnSource + Clone + 'static,
    S::Conn: tb_provider::strava::StravaStore + tb_provider::icu::IcuStore + Send + Sync + 'static,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
{
    /// The login (ADR-0014): one transaction — the provider layer's dispatch
    /// does the provider's login work (the scope check, the token exchange,
    /// the athlete wire-shape parsing, and the upsert), returning the
    /// identity and the admin flag.
    async fn login(&self, provider: Provider, code: &str, scope: &str) -> TbResult<(UserId, bool)> {
        let mut store = self.source.begin().await?;
        let result = tb_provider::dispatch::login(
            provider,
            code,
            scope,
            &tb_provider::EntityFacadeImpl,
            &mut store,
        )
        .await?;
        store.commit().await?;
        Ok(result)
    }

    /// The webhook intake (ADR-0014): one transaction — the provider
    /// layer's dispatch does the provider's intake work (the envelope parse,
    /// the secret verification, the event filtering, the timestamp parse, and
    /// the queue insert), then the wake per queued user (fire-and-forget).
    async fn accept_event(&self, provider: Provider, body: &str) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        let queued = tb_provider::dispatch::accept_event(provider, body, &mut store).await?;
        store.commit().await?;
        // Fire the in-memory wake signal (spec §4.4) per queued user: a pure
        // signal to a running executor — it never spawns one, and the DB
        // queue is the source of truth, so a missed wake only delays the
        // event to the next spawn.
        for user_id in queued {
            self.wake(user_id).await;
        }
        Ok(())
    }

    /// The per-user sync (ADR-0014): one transaction — the provider layer's
    /// dispatch reads the user's real provider in-transaction and enqueues a
    /// `Sync` event on their provider queue.
    async fn sync(
        &self,
        _session: Box<dyn Session>,
        user_id: UserId,
        event_time: i64,
    ) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        tb_provider::dispatch::sync(
            user_id,
            event_time,
            &tb_provider::EntityFacadeImpl,
            &mut store,
        )
        .await?;
        store.commit().await?;
        Ok(())
    }

    /// The deauthorization (ADR-0014): one transaction — the dispatch
    /// resolves the target's provider in-transaction, builds their session
    /// in-txn, then deauthorizes.
    async fn deauthorize(&self, mut session: Box<dyn Session>, tbid: UserId) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        tb_provider::dispatch::deauthorize(
            &mut *session,
            tbid,
            &tb_provider::EntityFacadeImpl,
            &mut store,
        )
        .await?;
        store.commit().await?;
        Ok(())
    }

    /// The user delete (ADR-0014): one transaction — the dispatch resolves
    /// the target's provider in-transaction, builds their session in-txn,
    /// then deletes.
    async fn delete_user(&self, mut session: Box<dyn Session>, tbid: UserId) -> TbResult<()> {
        let mut store = self.source.begin().await?;
        tb_provider::dispatch::delete_user(
            &mut *session,
            tbid,
            &tb_provider::EntityFacadeImpl,
            &mut store,
        )
        .await?;
        store.commit().await?;
        Ok(())
    }

    /// The activity redirect URL (ADR-0014): one transaction — the provider
    /// layer's `activity_url` op (which checks the activity's ownership), then
    /// the provider URL.
    async fn activity_url(
        &self,
        provider: Provider,
        session: Box<dyn Session>,
        id: ActivityId,
    ) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let uri = tb_provider::dispatch::activity_url(
            provider,
            session,
            id,
            &tb_provider::EntityFacadeImpl,
            &mut store,
        )
        .await?;
        store.commit().await?;
        Ok(uri)
    }

    /// The gear redirect URL (ADR-0014): one transaction — the provider
    /// layer's `gear_url` op (which checks the part's ownership). The one
    /// non-total op: the icu arm is the typed absence (`None`).
    async fn gear_url(
        &self,
        provider: Provider,
        session: Box<dyn Session>,
        id: PartId,
    ) -> TbResult<Option<String>> {
        let mut store = self.source.begin().await?;
        let uri = tb_provider::dispatch::gear_url(
            provider,
            session,
            id,
            &tb_provider::EntityFacadeImpl,
            &mut store,
        )
        .await?;
        store.commit().await?;
        Ok(uri)
    }

    /// The user redirect URL (ADR-0014): one transaction — the provider
    /// layer's `user_url` op for the target's tbid (no session). The icu arm
    /// builds the public athlete profile page.
    async fn user_url(&self, provider: Provider, id: UserId) -> TbResult<String> {
        let mut store = self.source.begin().await?;
        let uri = tb_provider::dispatch::user_url(provider, id, &mut store).await?;
        store.commit().await?;
        Ok(uri)
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
impl<S, P, D> Registry<S, P, D>
where
    S: TxnSource + Clone + 'static,
    P: ProviderLane<Store = S::Conn> + Clone + Send + Sync + 'static,
    D: DomainFacade<S::Conn> + Clone + Send + Sync + 'static,
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
