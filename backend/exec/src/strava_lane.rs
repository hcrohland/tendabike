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

//! The Strava provider lane: the [`ProviderLane`] impl for the Strava
//! driver. Lives in `tb_exec` (not `tb_strava`) because the crate graph is
//! `tb_domain` ← `tb_strava` ← `tb_exec` — `tb_strava` cannot depend on
//! `tb_exec` (circular), so the lane that bridges the loop's [`ProviderLane`]
//! seam to `tb_strava`'s `get_event`/`process` ops sits in the crate that
//! depends on both. `probe` delegates to `tb_strava::event::get_event` and
//! `process` to `tb_strava::event::process`.

use tb_domain::{Summary, UserId};
use tb_strava::StravaStore;
use tb_strava::event::{Event, QueueRead, get_event, process};

use crate::lane::{Probe, ProviderLane};

/// The Strava provider lane: the [`ProviderLane`] impl for the Strava driver.
/// The loop calls through it; a second provider (a new crate) implements
/// [`ProviderLane`] too, and the loop works with it unchanged.
#[derive(Clone, Copy, Debug, Default)]
pub struct StravaLane;

#[async_trait::async_trait]
impl ProviderLane for StravaLane {
    type Event = Event;

    /// Probe the Strava event queue (one transaction's housekeeping): read
    /// the next event, applying the queue's read-side rules (the rate-limit
    /// gate, the latest-per-object dedup). Delegates to
    /// [`tb_strava::event::get_event`].
    async fn probe<C: StravaStore>(&self, user_id: UserId, store: &mut C) -> Probe<Self::Event> {
        match get_event(user_id, store).await {
            Ok(QueueRead::Event(event)) => Probe::Event(event),
            Ok(QueueRead::Limited { until }) => Probe::Limited(until),
            Ok(QueueRead::Empty) => Probe::Empty,
            Err(_) => Probe::Error,
        }
    }

    /// Process the user's next queued Strava event (the work transaction):
    /// read the queue again (the accepted double read) and handle the event.
    /// Delegates to [`tb_strava::event::process`]. Returns the event's
    /// [`Summary`] on success, or `None` on a DB failure.
    async fn process<C: StravaStore + 'static>(
        &self,
        user_id: UserId,
        store: &mut C,
    ) -> Option<Summary> {
        process(user_id, &tb_provider::DomainFacadeImpl, store)
            .await
            .ok()
    }
}
