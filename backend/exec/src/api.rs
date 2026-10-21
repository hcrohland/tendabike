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
//! write, the stream subscription, the admin stats read, and the direct
//! ops (rescan, sync, the Strava plumbing).
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

use oauth2::RefreshToken;
use tokio::sync::{broadcast, watch};

use tb_domain::{
    ActivityId, ApiRead, ApiWrite, PartId, ReadOutcome, Summary, TbResult, UserId, WriteOutcome,
};
use tb_strava::{StravaStat, StravaUser, client::StravaAthleteInfo, event::InEvent};

use crate::ExecutorStatus;

/// The app's presentation-agnostic service API (spec #558 §2): 15 methods —
/// the five channel ops (`read`, `write`, `hydrate_gear`, `subscribe`,
/// `all_stats`) and the ten direct ops (spec #558 §5.2). Presentation
/// layers depend on this trait, not the concrete [`crate::Registry`].
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait::async_trait]
pub trait ServiceApi: Send + Sync {
    /// One per-user read on the user's executor (spec #558 §3): the loop
    /// runs it in its own transaction (read-only within it by spec) and
    /// resolves the read's data (a [`ReadOutcome`]); the caller knows which
    /// variant it sent and pattern-matches its own result.
    async fn read(&self, user_id: UserId, read: ApiRead) -> TbResult<ReadOutcome>;

    /// One per-user write on the user's executor (spec §6.2): the loop runs
    /// it in its own transaction and resolves the full [`WriteOutcome`]
    /// (the `Summary` — the frame — plus, for the descend, its match
    /// report, the response body). The oneshot wait is unbounded.
    async fn write(&self, user_id: UserId, write: ApiWrite) -> TbResult<WriteOutcome>;

    /// The hydration write (spec #558 §4): `update_gear` in the loop's own
    /// transaction with the loop's session, resolving with the created
    /// parts' `Summary` — the frame (the union of the parts it created,
    /// one entry per part, empty when nothing is new).
    async fn hydrate_gear(&self, user_id: UserId) -> TbResult<Summary>;

    /// Register a stream for `user_id` (spawning the executor if it is not
    /// running): a subscription to the frame channel plus a handle to the
    /// executor's fate ([`ExecutorStatus`]), so the stream can tell a clean
    /// reap from a dead executor (spec #558 §6.3).
    async fn subscribe(
        &self,
        user_id: UserId,
    ) -> TbResult<(
        broadcast::Receiver<Summary>,
        watch::Receiver<ExecutorStatus>,
    )>;

    /// The admin `GET /user/all` read (spec #558 §3.3): `get_all_stats` in
    /// the admin user's own channel, in the loop's own transaction.
    async fn all_stats(&self, user_id: UserId) -> TbResult<Vec<StravaStat>>;

    /// The admin rescan (spec #558 §5.2): one transaction
    /// (`Activity::rescan_all`), then every live executor stopped (the
    /// rescan's effect rides no frame — the clients re-hydrate on their
    /// native reconnect).
    async fn rescan(&self) -> TbResult<()>;

    /// The per-user admin sync (spec #558 §5.2): read the target's Strava
    /// id, enqueue a `Sync` event on the `strava_events` queue, wake the
    /// executor, and await that event's completion (unbounded — the spec'd
    /// behavior for the admin endpoint), with the queue-fallback read only
    /// on a lost broadcast.
    async fn sync(&self, user_id: UserId, event_time: i64) -> TbResult<()>;

    /// The all-users admin sync (spec #558 §5.2): one transaction
    /// (`sync_users`) queuing a `Sync` event per (non-disabled) user, then
    /// a wake per queued user (a running executor drains its queued sync
    /// immediately, matching the per-user variant). `user_id` narrows the
    /// sync to one user (the route's `?user_id=` query).
    async fn sync_all(
        &self,
        user_id: Option<UserId>,
        event_time: i64,
        migrate: bool,
    ) -> TbResult<()>;

    /// The login upsert (spec #558 §5.2): one transaction
    /// (`StravaUser::upsert` + `is_admin`), returning the identity and the
    /// admin flag. The caller keeps the query-parse, the scope check, the
    /// HMAC path, the outbound exchange, and the tower-session write.
    async fn login_from_token(
        &self,
        athlete: StravaAthleteInfo,
        refresh: Option<RefreshToken>,
    ) -> TbResult<(StravaUser, bool)>;

    /// The webhook accept (spec #558 §5.2): one transaction
    /// (`InEvent::accept` — the queue insert, or the athlete-deauth
    /// branch), then the wake if a user was queued (fire-and-forget).
    async fn accept_event(&self, event: InEvent) -> TbResult<()>;

    /// The gear redirect URL (spec #558 §5.2): the `tb_strava` `strava_url`
    /// op for `id`, gated on the caller's ownership (the caller's session
    /// built in-txn).
    async fn gear_url(&self, user: UserId, id: PartId) -> TbResult<String>;

    /// The activity redirect URL (spec #558 §5.2): the `tb_strava`
    /// `strava_url` op for `id`, gated on the caller's ownership (the
    /// caller's session built in-txn).
    async fn activity_url(&self, user: UserId, id: ActivityId) -> TbResult<String>;

    /// The user redirect URL (spec #558 §5.2): the `tb_strava` `strava_url`
    /// op for the target's tbid (no session: the route has no user context
    /// to gate on).
    async fn user_url(&self, id: UserId) -> TbResult<String>;

    /// The deauthorization (spec #558 §5.2): one transaction — the target's
    /// session built in-txn, then `tb_strava::user_deauthorize`. The
    /// existing queue race is preserved.
    async fn deauthorize(&self, tbid: UserId) -> TbResult<()>;

    /// The user delete (spec #558 §5.2): one transaction — the target's
    /// session built in-txn, then `tb_strava::user_delete`. The existing
    /// queue race is preserved.
    async fn delete_user(&self, tbid: UserId) -> TbResult<()>;
}
