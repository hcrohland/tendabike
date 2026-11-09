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

//! The Strava provider lane: the [`ProviderLane`] impl for the Strava driver.
//! Lives in `tb_strava` because the lane bridges the loop's [`ProviderLane`]
//! seam (in `tb_provider`) to `tb_strava`'s `get_event`/`process` ops — the
//! crate that owns those ops owns the lane. `probe` delegates to
//! [`crate::event::get_event`] and `process` to [`crate::event::process`]. The
//! lane is generic over the store type `S` (the provider's full store — the
//! transaction's connection): `type Store = S`, so the loop's transaction
//! connection ties to `S` and `probe`/`process` operate on it.

use std::marker::PhantomData;

use tb_domain::{PartId, Summary, TbResult, UserId, UserStat};
use tb_provider::{Probe, ProviderLane};

use crate::StravaStore;
use crate::event::{Event, QueueRead, get_event, process};

/// The Strava provider lane: the [`ProviderLane`] impl for the Strava driver.
/// The loop calls through it; a second provider (a new crate) implements
/// [`ProviderLane`] too, and the loop works with it unchanged.
///
/// Generic over the store type `S` (the provider's full store — the
/// transaction's connection): `type Store = S`, so the loop's transaction
/// connection ties to `S`. `S` is the connection the loop's transaction
/// source opens (the Postgres connection in production, the test double in
/// the suites), which is why the lane is generic over it rather than naming
/// a concrete store.
///
/// The marker-field impls (`Clone`/`Copy`/`Debug`/`Default`) are written by
/// hand rather than derived: a derive would add an `S: Clone`/`S: Debug`
/// bound, but the lane holds no `S` data (only a `PhantomData<S>` marker), so
/// it is `Clone`/`Copy`/`Debug`/`Default` for any `S` — including a database
/// connection, which is neither `Clone` nor `Debug`.
pub struct StravaLane<S: StravaStore> {
    _store: PhantomData<S>,
}

impl<S: StravaStore> Clone for StravaLane<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: StravaStore> Copy for StravaLane<S> {}

impl<S: StravaStore> std::fmt::Debug for StravaLane<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StravaLane").finish()
    }
}

impl<S: StravaStore> Default for StravaLane<S> {
    fn default() -> Self {
        Self {
            _store: PhantomData,
        }
    }
}

// `StravaLane<S>` holds no `S` data (only a `PhantomData<S>` marker), so it is
// safe to share across threads even when `S` is not `Sync` (a database
// connection, like the Postgres one, is `Send` but not `Sync`). The
// auto-derived `Sync` would require `S: Sync`; this sound `unsafe impl`
// provides the unconditional `Sync` the `ProviderLane: Send + Sync` bound
// needs.
unsafe impl<S: StravaStore> Sync for StravaLane<S> {}

impl<S: StravaStore> StravaLane<S> {
    /// The lane for a store of type `S`.
    pub fn new() -> Self {
        Self {
            _store: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<S> ProviderLane for StravaLane<S>
where
    S: StravaStore + Send + 'static,
{
    type Event = Event;
    type Store = S;

    /// Probe the Strava event queue (one transaction's housekeeping): read
    /// the next event, applying the queue's read-side rules (the rate-limit
    /// gate, the latest-per-object dedup). Delegates to
    /// [`crate::event::get_event`].
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<Self::Event> {
        match get_event(user_id, store).await {
            Ok(QueueRead::Event(event)) => Probe::Event(event),
            Ok(QueueRead::Limited { until }) => Probe::Limited(until),
            Ok(QueueRead::Empty) => Probe::Empty,
            Err(_) => Probe::Error,
        }
    }

    /// Process the user's next queued Strava event (the work transaction):
    /// read the queue again (the accepted double read) and handle the event.
    /// Delegates to [`crate::event::process`]. Returns the event's
    /// [`Summary`] on success, or `None` on a DB failure.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        process(user_id, &tb_provider::DomainFacadeImpl, store)
            .await
            .ok()
    }

    /// Queue a sync event on the Strava event queue (the onboarding cutover's
    /// provider side): the loop runs it in the write's own transaction, so the
    /// queued event commits with the domain half. Delegates to
    /// [`crate::event::insert_sync`].
    async fn insert_sync(
        &self,
        user_id: UserId,
        time: i64,
        auto: bool,
        store: &mut S,
    ) -> TbResult<()> {
        crate::event::insert_sync(user_id, time, auto, store).await
    }

    /// Import the user's Strava gear, returning the created part ids (the
    /// hydration write's provider side): the loop runs it in its own
    /// transaction and reads the parts it created for the frame. Delegates to
    /// [`crate::user::StravaUser::update_gear`].
    async fn update_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        crate::user::StravaUser::update_gear(user_id, &tb_provider::DomainFacadeImpl, store).await
    }

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side): the domain [`UserStat`] — the stat plus the provider's
    /// queued-event count and disabled flag. Delegates to
    /// [`crate::user::get_all_stats`].
    async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>> {
        crate::user::get_all_stats(&tb_provider::DomainFacadeImpl, store).await
    }
}
