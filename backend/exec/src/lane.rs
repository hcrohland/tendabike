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
//! transaction), and never names a concrete provider. `tb_strava` implements
//! it; a second provider (a new crate) implements it too, and the loop works
//! with it unchanged. The [`Probe`] outcome is generic over the provider's
//! event type, so the loop's select and reclaim policy are provider-agnostic.

use tb_domain::{Summary, UserId};
use tb_strava::StravaStore;

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
/// `tb_strava` implements it; a second provider (a new crate implementing
/// [`ProviderLane`] + its data store) plugs in without touching the loop.
#[async_trait::async_trait]
pub trait ProviderLane: Send + Sync {
    /// The provider's event type.
    type Event: Clone + Send + Sync + std::fmt::Debug + 'static;

    /// Probe the provider's event queue for the user (one transaction): read
    /// the next event, applying the queue's read-side rules (the rate-limit
    /// gate, the latest-per-object dedup). The housekeeping (the expired
    /// `Stop`, the dropped older duplicates) commits or rolls back with the
    /// caller's transaction.
    async fn probe<C: StravaStore>(&self, user_id: UserId, store: &mut C) -> Probe<Self::Event>;

    /// Process the user's next queued event (the work transaction): read the
    /// queue again (the accepted double read) and handle the event. Returns
    /// the event's [`Summary`] on success, or `None` on a DB failure (the
    /// caller rolls back and backs off).
    async fn process<C: StravaStore>(&self, user_id: UserId, store: &mut C) -> Option<Summary>;
}
