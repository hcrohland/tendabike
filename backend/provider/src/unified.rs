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

//! The unified provider lane (ADR-0014): the [`ProviderLane`] impl the
//! executor loop runs in production — the third impl of the [`Lane<S, E, F>`]
//! seam, beside the two per-provider lanes.
//!
//! The loop is generic over a single [`ProviderLane`] type, but a user has
//! exactly one provider (the `users.provider` column, immutable). This lane
//! wraps both provider lanes — [`StravaLane`] and [`IcuLane`] — and selects
//! between them per call from the user's `provider` column (a typed switch
//! on [`Provider`]), so the loop's control flow (the biased select, the
//! backoff, the idle policy, the frame push) is unchanged and
//! provider-agnostic.
//!
//! The unified event type ([`LaneEvent`]) is the loop's `P::Event`: an enum
//! over the two providers' event rows, so the loop's `Select`, the
//! consumed-event channel, and the `run_event` dispatch are all
//! provider-agnostic.

use tb_domain::{ActivityId, PartId, Session, Summary, TbResult, UserId, UserStat};

use crate::Provider;
use crate::icu::{IcuLane, IcuStore, event::IcuEvent};
use crate::strava::{StravaLane, StravaStore, event::Event as StravaEvent};
use crate::{EntityFacadeImpl, Lane, Probe, ProviderLane};

/// The unified provider event the per-user lane reports: the two providers'
/// event rows under one type, so the loop's `P::Event` is provider-agnostic
/// (spec #764: the `Message` enum's `Strava`/`Icu` arms).
#[derive(Debug, Clone, PartialEq)]
pub enum LaneEvent {
    /// A Strava event row (the `strava_events` queue).
    Strava(StravaEvent),
    /// An intervals.icu event row (the `icu_events` queue).
    Icu(IcuEvent),
}

/// A Strava event is a `LaneEvent`; a non-Strava one is `None` (the admin
/// `sync` op is Strava-only, so it correlates only the Strava arm).
impl From<LaneEvent> for Option<StravaEvent> {
    fn from(event: LaneEvent) -> Self {
        match event {
            LaneEvent::Strava(event) => Some(event),
            LaneEvent::Icu(_) => None,
        }
    }
}

/// The unified provider lane: [`Lane<S, LaneEvent, EntityFacadeImpl>`](Lane) —
/// the third [`ProviderLane`] impl (ADR-0014), beside the two per-provider
/// lanes. It wraps both provider lanes and selects between them per call from
/// the user's `provider` column (a typed switch on [`Provider`]); the loop is
/// generic over this single type and never names a concrete provider.
///
/// Generic over the store type `S` (the transaction's connection):
/// `type Store = S`, so the loop's transaction connection ties to `S` and
/// `probe`/`process` operate on it. `S` must be a complete store for both
/// providers (`StravaStore + IcuStore`), which also carries the domain
/// `Store` (the `user_get` the per-call selection reads).
///
/// The unified lane is production-only: it is concrete over the
/// [`EntityFacadeImpl`] (no mock injection) — tests target the per-provider
/// lanes directly.
pub type UnifiedLane<S> = Lane<S, LaneEvent, EntityFacadeImpl>;

impl<S> Lane<S, LaneEvent, EntityFacadeImpl>
where
    S: StravaStore + IcuStore,
{
    /// The user's provider, read from the `users.provider` column (through
    /// the owning entity's [`UserId::provider`] read, ADR-0010) and parsed
    /// into the typed [`Provider`] (the single dispatch mechanism, ADR-0014).
    /// A read failure is a DB failure the caller classifies.
    async fn provider_of(store: &mut S, user_id: UserId) -> TbResult<Provider> {
        let s = user_id.provider(store).await?;
        Provider::try_from(s.as_str())
    }
}

#[async_trait::async_trait]
impl<S> ProviderLane for Lane<S, LaneEvent, EntityFacadeImpl>
where
    S: StravaStore + IcuStore + Send + 'static,
{
    type Event = LaneEvent;
    type Store = S;
    type Facade = EntityFacadeImpl;

    fn facade(&self) -> &EntityFacadeImpl {
        &self.facade
    }

    /// Probe the user's provider event queue (one transaction's
    /// housekeeping): read the user's `provider` column, then delegate to
    /// that provider's lane. The returned event is wrapped in [`LaneEvent`].
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<LaneEvent> {
        let provider = match Self::provider_of(store, user_id).await {
            Ok(provider) => provider,
            Err(_) => return Probe::Error,
        };
        match provider {
            Provider::Icu => match IcuLane::new(EntityFacadeImpl).probe(user_id, store).await {
                Probe::Event(event) => Probe::Event(LaneEvent::Icu(event)),
                Probe::Limited(until) => Probe::Limited(until),
                Probe::Empty => Probe::Empty,
                Probe::Error => Probe::Error,
            },
            Provider::Strava => match StravaLane::new(EntityFacadeImpl)
                .probe(user_id, store)
                .await
            {
                Probe::Event(event) => Probe::Event(LaneEvent::Strava(event)),
                Probe::Limited(until) => Probe::Limited(until),
                Probe::Empty => Probe::Empty,
                Probe::Error => Probe::Error,
            },
        }
    }

    /// Process the user's next queued event (the work transaction): read the
    /// user's `provider` column, then delegate to that provider's lane.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        let provider = match Self::provider_of(store, user_id).await {
            Ok(provider) => provider,
            Err(_) => return None,
        };
        match provider {
            Provider::Icu => IcuLane::new(EntityFacadeImpl).process(user_id, store).await,
            Provider::Strava => {
                StravaLane::new(EntityFacadeImpl)
                    .process(user_id, store)
                    .await
            }
        }
    }

    /// Queue a sync event on the user's provider event queue (the onboarding
    /// cutover's provider side): delegate to the user's provider's lane.
    async fn insert_sync(
        &self,
        user_id: UserId,
        time: i64,
        auto: bool,
        store: &mut S,
    ) -> TbResult<()> {
        let provider = Self::provider_of(store, user_id).await?;
        match provider {
            Provider::Icu => {
                IcuLane::new(EntityFacadeImpl)
                    .insert_sync(user_id, time, auto, store)
                    .await
            }
            Provider::Strava => {
                StravaLane::new(EntityFacadeImpl)
                    .insert_sync(user_id, time, auto, store)
                    .await
            }
        }
    }

    /// Import the user's gear from their provider (the hydration write's
    /// provider side): delegate to the user's provider's lane.
    async fn update_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        let provider = Self::provider_of(store, user_id).await?;
        match provider {
            Provider::Icu => {
                IcuLane::new(EntityFacadeImpl)
                    .update_gear(user_id, store)
                    .await
            }
            Provider::Strava => {
                StravaLane::new(EntityFacadeImpl)
                    .update_gear(user_id, store)
                    .await
            }
        }
    }

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side): the union of the Strava users' stats and the icu users' stats
    /// (each user's stat is sourced from their own provider).
    async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>> {
        let mut stats = StravaLane::new(EntityFacadeImpl).all_stats(store).await?;
        stats.extend(IcuLane::new(EntityFacadeImpl).all_stats(store).await?);
        Ok(stats)
    }

    /// The per-user half of [`ProviderLane::activity_url`]: read the user's
    /// `provider` column, then delegate to that provider's lane.
    async fn activity_url(
        &self,
        id: ActivityId,
        session: Box<dyn Session>,
        store: &mut S,
    ) -> TbResult<String> {
        let provider = Self::provider_of(store, session.user_id()).await?;
        match provider {
            Provider::Icu => {
                IcuLane::new(EntityFacadeImpl)
                    .activity_url(id, session, store)
                    .await
            }
            Provider::Strava => {
                StravaLane::new(EntityFacadeImpl)
                    .activity_url(id, session, store)
                    .await
            }
        }
    }
}
