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

//! The app's presentation-agnostic service API (ADR-0007, executable spec
//! #558 §2): the exec-owned surface a presentation layer (the web layer,
//! the named second driver, a CLI) calls — reads, writes, the hydration
//! write, the stream subscription, the admin stats read, and the
//! provider-neutral direct ops (`rescan`, `subscribe_shop`,
//! `trigger_processing`).
//!
//! The [`crate::Registry`] implements it; presentation layers hold
//! `Arc<dyn ServiceApi>` and never name a seam type (`TxnSource`/`Txn`
//! stay `pub` in this crate because `tb_sqlx` must implement them — the
//! presentation layer simply never names them). In the seam vocabulary
//! this makes the seam real: two adapters (the concrete `Registry` in
//! production, the recording stub in the web layer's suite), and the
//! interface is the test surface — callers and tests cross the same seam.
//!
//! The inclusion rule (spec §2.1): only what a presentation layer calls.
//! Off the trait: `wake`/`stop_all`/`events`/`send_frame`/`status` —
//! lifecycle internals (the direct ops do their own waking internally) and
//! the test helpers. The heartbeat cadence is not a trait method: the SSE
//! `KeepAlive` wiring uses [`crate::HEARTBEAT_INTERVAL`].
//!
//! `#[async_trait]`; object-safe for `Arc<dyn ServiceApi>`, which is what
//! keeps the web layer's `AppState` non-generic.

use tokio::sync::{broadcast, watch};

use tb_domain::{
    ApiRead, ApiWrite, ReadOutcome, Session, ShopId, Summary, TbResult, UserId, UserStat,
    WriteOutcome,
};

use crate::ExecutorStatus;

/// The app's presentation-agnostic service API (spec #558 §2): the generic
/// channel ops (`read`, `write`, `hydrate_gear`, `subscribe`, `all_stats`)
/// plus the provider-neutral direct ops (`rescan`, `subscribe_shop`,
/// `trigger_processing`). Presentation layers depend on this trait, not the
/// concrete [`crate::Registry`]. The provider-specific ops live in
/// [`crate::providerservice::ProviderService`] — a second provider never
/// reshapes this trait.
///
/// Every method that takes a `session` parameter threads the acting
/// [`Session`] through the executor to the downstream domain functions it
/// runs: the executor hands it to the op in its own transaction. The
/// session's `shop()` is the frame's delivery context (it routes a committed
/// `Summary`) and scopes a read's `Summary`.
#[cfg_attr(feature = "test-support", mockall::automock)]
#[async_trait::async_trait]
pub trait ServiceApi: Send + Sync {
    /// One per-user read on the user's executor (spec #558 §3): the loop
    /// runs it in its own transaction (read-only within it by spec) and
    /// resolves the read's data (a [`ReadOutcome`]); the caller knows which
    /// variant it sent and pattern-matches its own result.
    async fn read(&self, session: Box<dyn Session>, read: ApiRead) -> TbResult<ReadOutcome>;

    /// One per-user write on the user's executor (spec §6.2): the loop runs
    /// it in its own transaction and resolves the full [`WriteOutcome`]
    /// (the `Summary` — the frame — plus, for the descend, its match
    /// report, the response body). The oneshot wait is unbounded.
    async fn write(&self, session: Box<dyn Session>, write: ApiWrite) -> TbResult<WriteOutcome>;

    /// The hydration write (spec #558 §4): `update_gear` in the loop's own
    /// transaction, resolving with the created parts' `Summary` — the frame
    /// (the union of the parts it created, one entry per part, empty when
    /// nothing is new).
    async fn hydrate_gear(&self, session: Box<dyn Session>) -> TbResult<Summary>;

    /// Register a stream for the user (spawning the executor if it is not
    /// running): a subscription to the frame channel plus a handle to the
    /// executor's fate ([`ExecutorStatus`]), so the stream can tell a clean
    /// reap from a dead executor (spec #558 §6.3).
    async fn subscribe(
        &self,
        session: Box<dyn Session>,
    ) -> TbResult<(
        broadcast::Receiver<Summary>,
        watch::Receiver<ExecutorStatus>,
    )>;

    /// The admin `GET /user/all` read (spec #558 §3.3): `get_all_stats` in
    /// the admin user's own channel, in the loop's own transaction.
    async fn all_stats(&self, session: Box<dyn Session>) -> TbResult<Vec<UserStat>>;

    /// The admin rescan (spec #558 §5.2): one transaction
    /// (`Activity::rescan_all`), then every live executor stopped (the
    /// rescan's effect rides no frame — the clients re-hydrate on their
    /// native reconnect). Provider-neutral: a domain op, not a provider
    /// concept (ADR-0014).
    async fn rescan(&self, session: Box<dyn Session>) -> TbResult<()>;

    /// Register a shop stream for `shop_id` (spec #517): the caller must be
    /// the shop owner (a non-owner is `Forbidden`, which the web layer maps to
    /// a 403); the channel is created on demand and the returned receiver is a
    /// subscription to it. Provider-neutral: a shop stream, not a provider
    /// concept (ADR-0014).
    async fn subscribe_shop(
        &self,
        user_id: UserId,
        shop_id: ShopId,
    ) -> TbResult<broadcast::Receiver<Summary>>;

    /// Trigger processing for all (or one) users (spec #558 §5.2): one
    /// transaction (`sync_users`) queuing a `Sync` event per (non-disabled)
    /// user, then — per queued user — spawn the executor (a no-op if it is
    /// already running) and wake it, so the queue drains even for users whose
    /// executor is not running (offline). `user_id` narrows the trigger to one
    /// user (the route's `?user_id=` query). Provider-neutral at the interface
    /// (ADR-0014).
    async fn trigger_processing(
        &self,
        session: Box<dyn Session>,
        user_id: Option<UserId>,
        event_time: i64,
        migrate: bool,
    ) -> TbResult<()>;
}
