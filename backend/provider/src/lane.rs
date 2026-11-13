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

//! The provider lane: the narrow seam the executor loop calls through to
//! probe and process a provider's event queue. The loop is generic over this
//! trait: it calls `probe` to read the provider's queue (one transaction's
//! housekeeping) and `process` to handle a queued event (the work
//! transaction), and never names a concrete provider. Each provider
//! implements it for its own lane (e.g. the Strava module in `tb_provider`);
//! a second provider (a new feature module) implements it too, and the loop
//! works with it unchanged. The
//! [`Probe`] outcome is generic over the provider's event type, so the
//! loop's select and reclaim policy are provider-agnostic.

use std::marker::PhantomData;

use tb_domain::{ActivityId, PartId, Session, Summary, TbResult, UserId, UserStat};

/// The provider lane marker: a `PhantomData` over the store type `S` and the
/// provider's event type `E`. Written once in the kit; each provider re-exports
/// a type alias (e.g. `StravaLane<S> = Lane<S, StravaEvent>`) so existing names
/// keep working. The distinct `E` keeps the per-provider [`ProviderLane`] impls
/// apart (a plain `Lane<S>` would hit conflicting implementations, since the
/// production store implements both store traits).
///
/// The marker-field impls (`Clone`/`Copy`/`Debug`/`Default`) are written by
/// hand rather than derived: a derive would add an `S: Clone`/`S: Debug`
/// bound, but the lane holds no `S` or `E` data (only `PhantomData` markers),
/// so it is `Clone`/`Copy`/`Debug`/`Default` for any `S` and `E` — including
/// a database connection, which is neither `Clone` nor `Debug`.
pub struct Lane<S, E> {
    _store: PhantomData<S>,
    _event: PhantomData<E>,
}

impl<S, E> Clone for Lane<S, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, E> Copy for Lane<S, E> {}

impl<S, E> std::fmt::Debug for Lane<S, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lane").finish()
    }
}

impl<S, E> Default for Lane<S, E> {
    fn default() -> Self {
        Self {
            _store: PhantomData,
            _event: PhantomData,
        }
    }
}

// `Lane<S, E>` holds no `S` or `E` data (only `PhantomData` markers), so it is
// safe to share across threads even when `S` is not `Sync` (a database
// connection, like the Postgres one, is `Send` but not `Sync`). The
// auto-derived `Sync` would require `S: Sync`; this sound `unsafe impl`
// provides the unconditional `Sync` the `ProviderLane: Send + Sync` bound
// needs.
unsafe impl<S, E> Sync for Lane<S, E> {}

impl<S, E> Lane<S, E> {
    /// The lane for a store of type `S` and event type `E`.
    pub fn new() -> Self {
        Self {
            _store: PhantomData,
            _event: PhantomData,
        }
    }
}

/// The outcome of one probe of the provider's event queue (one transaction).
/// Generic over the provider's event type, so the loop's select and reclaim
/// policy are provider-agnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum Probe<E> {
    /// An event was queued; the work transaction will process it.
    Event(E),
    /// The global rate-limit `Stop` is active until this unix time: the loop
    /// sleeps until the deadline as a branch of the biased select (spec
    /// #446 §4.6) — API writes are never delayed by the backoff.
    Limited(i64),
    /// The queue holds nothing interesting for this user.
    Empty,
    /// The probe failed (its transaction rolled back); the queue's state is
    /// unknown.
    Error,
}

/// The narrow provider seam the executor loop calls through: `probe` reads
/// the provider's event queue (one transaction's housekeeping), and
/// `process` handles a queued event (the work transaction). The loop is
/// generic over this trait and never names a concrete provider.
///
/// Beyond the queue's two-step (`probe`/`process`), the lane carries the
/// provider-specific ops the loop drives in its own transaction — `insert_sync`
/// (the onboarding cutover's provider side), `update_gear` (the hydration
/// write), and `all_stats` (the admin read). These are provider-specific: a
/// second provider implements them for its own domain.
///
/// Each provider module implements it for its own lane; a second provider (a
/// new feature module implementing [`ProviderLane`] + its data store) plugs in
/// without touching the loop.
#[async_trait::async_trait]
pub trait ProviderLane: Send + Sync {
    /// The provider's event type.
    type Event: Clone + Send + Sync + std::fmt::Debug + 'static;

    /// The provider's full store: the transaction's connection the loop's
    /// transaction source opens. The loop ties its transaction connection to
    /// this type (`T::Conn = Self::Store`), so `probe` and `process` operate
    /// on the loop's own transaction connection — the lane never names a
    /// concrete provider's store.
    type Store;

    /// Probe the provider's event queue for the user (one transaction): read
    /// the next event, applying the queue's read-side rules (the rate-limit
    /// gate, the latest-per-object dedup). The housekeeping (the expired
    /// `Stop`, the dropped older duplicates) commits or rolls back with the
    /// caller's transaction.
    async fn probe(&self, user_id: UserId, store: &mut Self::Store) -> Probe<Self::Event>;

    /// Process the user's next queued event (the work transaction): read the
    /// queue again (the accepted double read) and handle the event. Returns
    /// the event's [`Summary`] on success, or `None` on a DB failure (the
    /// caller rolls back and backs off).
    async fn process(&self, user_id: UserId, store: &mut Self::Store) -> Option<Summary>;

    /// Queue a sync event on the provider's event queue (the onboarding
    /// cutover's provider side, spec #558 §5.1): the loop runs it in the
    /// write's own transaction, so the queued event commits with the domain
    /// half. `auto` is the migration flag — a `true` sync is the one-time
    /// onboarding backfill, a `false` sync a routine one.
    async fn insert_sync(
        &self,
        user_id: UserId,
        time: i64,
        auto: bool,
        store: &mut Self::Store,
    ) -> TbResult<()>;

    /// Import the user's gear from the provider, returning the created part
    /// ids (the hydration write's provider side, spec #558 §4): the loop runs
    /// it in its own transaction and reads the parts it created for the frame.
    async fn update_gear(&self, user_id: UserId, store: &mut Self::Store) -> TbResult<Vec<PartId>>;

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side, spec #558 §3.3): the domain [`UserStat`] — the stat plus the
    /// provider's queued-event count and disabled flag.
    async fn all_stats(&self, store: &mut Self::Store) -> TbResult<Vec<UserStat>>;

    /// The activity redirect URL (the `/source/activities/{id}` direct op's
    /// provider side, spec #558 §5.2): the provider's URL for `id`, gated on
    /// the acting session's ownership of the activity (the read is the
    /// ownership check). The registry runs it in its own transaction and
    /// delegates here — the dispatch and the per-provider URL building live
    /// on the lane, not in the registry.
    async fn activity_url(
        &self,
        id: ActivityId,
        session: Box<dyn Session>,
        store: &mut Self::Store,
    ) -> TbResult<String>;
}
