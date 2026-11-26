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
//! transaction), and never names a concrete provider. The lane is generic
//! over the provider marker [`ProviderSpec`](crate::ProviderSpec) and the
//! [`Provider`](crate::driver::Provider) driver: it is bounded on the single
//! [`Store`](crate::store::Store) interface and drives the `Provider` trait
//! methods, so it is provider-agnostic. The unified lane — the seam's single
//! impl (ADR-0014) — resolves the user's provider internally, in `probe`
//! (the [`provider_of`](crate::provider_of) lookup + a typed switch on the
//! [`Provider`](crate::Provider) enum); the registry and the loop never name
//! a concrete provider. The [`Probe`] outcome is generic over the lane's
//! event type.

use std::marker::PhantomData;

use crate::facade::{PartId, Summary, TbResult, UserId};

use crate::EntityFacade;

/// The provider lane: a `PhantomData` over the provider marker `P`, the store
/// type `S`, and the driver `D`, plus the lane's entity-op facade `F` (the
/// provider's door into the domain, which the lane's ops forward domain work
/// through). The lane is generic over the provider marker and driver, so it
/// is provider-agnostic: it is bounded on the single [`Store<P>`] interface
/// and drives the [`Provider<P>`] trait methods.
///
/// The store marker is a `PhantomData<fn() -> S>` (a function pointer, which
/// is unconditionally `Send + Sync`) rather than a `PhantomData<S>`: the
/// `ProviderLane: Send + Sync` bound must hold for the lane independent of
/// the store type. The `Debug` impl is written by hand (just `Lane`): a
/// derive would require `S: Debug`/`F: Debug`, but a database connection is
/// not `Debug`.
pub struct Lane<P, S, F, D> {
    _spec: PhantomData<P>,
    _store: PhantomData<fn() -> S>,
    _driver: PhantomData<D>,
    pub(crate) facade: F,
}

impl<P, S, F: Clone, D> Clone for Lane<P, S, F, D> {
    fn clone(&self) -> Self {
        Self {
            _spec: PhantomData,
            _store: PhantomData,
            _driver: PhantomData,
            facade: self.facade.clone(),
        }
    }
}

impl<P, S, F, D> std::fmt::Debug for Lane<P, S, F, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lane").finish()
    }
}

impl<P, S, F, D> Lane<P, S, F, D> {
    /// The lane for a provider marker `P`, store of type `S`, entity-op
    /// facade `F`, and driver `D`.
    pub fn new(facade: F) -> Self {
        Self {
            _spec: PhantomData,
            _store: PhantomData,
            _driver: PhantomData,
            facade,
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
    /// The user no longer exists (deleted): the loop should exit cleanly.
    UserGone,
    /// The probe failed (its transaction rolled back); the queue's state is
    /// unknown.
    Error,
}

/// One outcome of a read of a provider's event queue (one transaction): what
/// is interesting for the user right now, and when the queue is paused.
/// Generic over the driver's event type. The input to the probe outcome
/// ([`Probe`]): the lane's `probe` maps it to the `Probe` the loop consumes,
/// so the queue-read concept and its translation live together in the kit,
/// not per driver.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QueueRead<E> {
    /// An event to process (the read-side rules already applied).
    Event(E),
    /// The driver's rate-limit budget is exhausted until this unix time: the
    /// queue is paused, and the caller sleeps until the window boundary.
    Limited { until: i64 },
    /// The queue holds nothing for this user.
    Empty,
}

impl<E> QueueRead<E> {
    /// The probe outcome this queue read maps to, wrapping the carried event
    /// with `into` (the lane's unified-event arm). Total over the three queue
    /// read outcomes; the user-gone and error probe outcomes are the
    /// provider-identity lookup's, not the queue read's.
    pub(crate) fn into_probe<W>(self, into: impl FnOnce(E) -> W) -> Probe<W> {
        match self {
            QueueRead::Event(event) => Probe::Event(into(event)),
            QueueRead::Limited { until } => Probe::Limited(until),
            QueueRead::Empty => Probe::Empty,
        }
    }
}

/// The narrow provider seam the executor loop calls through: `probe` reads
/// the provider's event queue (one transaction's housekeeping), and
/// `process` handles a queued event (the work transaction). The loop is
/// generic over this trait and never names a concrete provider.
///
/// Beyond the queue's two-step (`probe`/`process`), the lane carries the
/// provider-specific ops the loop drives in its own transaction — `import_gear`
/// and `queue_sync` (the onboarding's two steps: the gear import and the queued
/// activity sync).
///
/// The lane is generic over the provider marker and driver: it is bounded on
/// the single [`Store<P>`](crate::store::Store) interface and drives the
/// [`Provider<P>`](crate::driver::Provider) trait methods, so it is
/// provider-agnostic. The unified lane — the seam's single impl (ADR-0014) —
/// resolves the user's provider internally, in `probe` (the
/// [`provider_of`](crate::provider_of) lookup + a typed switch on the
/// [`Provider`](crate::Provider) enum); the registry and the loop never name
/// a concrete provider. The lane carries its entity-op facade
/// ([`Facade`](ProviderLane::Facade) — the provider's door into the domain,
/// ADR-0013), which its ops forward domain work through, so the impl bounds
/// on the single store interface, not the full domain store.
#[async_trait::async_trait]
pub trait ProviderLane: Send + Sync {
    /// The provider's event type.
    type Event: Clone + Send + Sync + std::fmt::Debug + 'static;

    /// The provider's full store: the transaction's connection the loop's
    /// transaction source opens. The loop ties its transaction connection to
    /// this type (`T::Conn = Self::Store`), so `probe` and `process` operate
    /// on the loop's own transaction connection — the lane never names a
    /// concrete provider's store. `Send + Sync` (the entity-op facade's
    /// store parameter bound): the production connection (a database
    /// transaction) carries it.
    type Store: Send + Sync;

    /// The lane's entity-op facade: the provider's door into the domain
    /// ([`EntityFacade`]), which the lane's ops forward domain work through.
    type Facade: EntityFacade<Self::Store>;

    /// The lane's entity-op facade.
    fn facade(&self) -> &Self::Facade;

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

    /// Import the user's gear (bikes + shoes) from the provider (the
    /// onboarding's first step, spec #829): the loop runs it in the write's
    /// own transaction, and it commits with the domain half. Returns the
    /// created part ids — the onboarding response lists them, and the loop
    /// reads the parts back for the frame. No activity sync is queued: the
    /// user decides on that next ([`queue_sync`](ProviderLane::queue_sync)).
    async fn import_gear(&self, user_id: UserId, store: &mut Self::Store) -> TbResult<Vec<PartId>>;

    /// Queue the activity sync on the provider's event queue (the onboarding's
    /// second step, spec #829): the loop runs it in the write's own
    /// transaction, so the queued event commits with the domain half. No gear
    /// import — that was the first step
    /// ([`import_gear`](ProviderLane::import_gear)).
    async fn queue_sync(&self, user_id: UserId, time: i64, store: &mut Self::Store)
    -> TbResult<()>;
}

#[cfg(test)]
mod tests {
    use super::{Probe, QueueRead};

    /// The queue-read-to-probe mapping is total over the three queue-read
    /// outcomes: the event is wrapped, the pause carries its deadline, and
    /// empty is empty. Pure — no store, no mocks.
    #[test]
    fn into_probe_maps_all_three_outcomes() {
        assert_eq!(
            QueueRead::<i32>::Event(42).into_probe(|e| e),
            Probe::Event(42)
        );
        assert_eq!(
            QueueRead::<i32>::Limited { until: 123 }.into_probe(|e| e),
            Probe::Limited(123)
        );
        assert_eq!(QueueRead::<i32>::Empty.into_probe(|e| e), Probe::Empty);
    }
}
