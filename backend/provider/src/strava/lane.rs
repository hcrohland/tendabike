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
//! Lives in the Strava module because the lane bridges the loop's
//! [`ProviderLane`] seam to the module's `get_event`/`process` ops — the code
//! that owns those ops owns the lane. `probe` delegates to
//! [`crate::event::get_event`] and `process` to [`crate::event::process`]. The
//! lane is generic over the store type `S` (the provider's full store — the
//! transaction's connection): `type Store = S`, so the loop's transaction
//! connection ties to `S` and `probe`/`process` operate on it.

use crate::{Lane, Probe, ProviderActivityId, ProviderLane};
use tb_domain::{ActivityId, Error, PartId, Session, Summary, TbResult, UserId, UserStat};

use super::StravaStore;
use super::event::{Event, QueueRead, get_event, process};

/// The Strava provider lane: [`Lane<S, Event>`](Lane) — the [`ProviderLane`]
/// impl for the Strava driver. The distinct event type `Event` keeps this impl
/// apart from the icu lane's. The loop calls through it; a second provider (a
/// new feature module) implements [`ProviderLane`] too, and the loop works
/// with it unchanged.
pub type StravaLane<S> = Lane<S, Event>;

#[async_trait::async_trait]
impl<S> ProviderLane for Lane<S, Event>
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
        process(user_id, &crate::EntityFacadeImpl, store).await.ok()
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
        super::event::insert_sync(user_id, time, auto, store).await
    }

    /// Import the user's Strava gear, returning the created part ids (the
    /// hydration write's provider side): the loop runs it in its own
    /// transaction and reads the parts it created for the frame. Delegates to
    /// [`crate::user::StravaUser::update_gear`].
    async fn update_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        super::user::StravaUser::update_gear(user_id, &crate::EntityFacadeImpl, store).await
    }

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side): the domain [`UserStat`] — the stat plus the provider's
    /// queued-event count and disabled flag. Delegates to
    /// [`crate::user::get_all_stats`].
    async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>> {
        super::user::get_all_stats(&crate::EntityFacadeImpl, store).await
    }

    /// The Strava half of [`ProviderLane::activity_url`]: the `strava:` URL
    /// for `id`, gated on the acting session's ownership of the activity.
    /// Delegates to [`crate::activity::strava_url`].
    async fn activity_url(
        &self,
        id: ActivityId,
        session: Box<dyn Session>,
        store: &mut S,
    ) -> TbResult<String> {
        let ProviderActivityId::Strava(raw) = ProviderActivityId::try_from(id)? else {
            return Err(Error::BadRequest("not a strava activity".to_string()));
        };
        super::activity::strava_url(raw, session, &crate::EntityFacadeImpl, store).await
    }
}
