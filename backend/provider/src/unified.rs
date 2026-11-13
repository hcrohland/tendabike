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
//! executor loop runs in production — the seam's single impl.
//!
//! The loop is generic over a single [`ProviderLane`] type, but a user has
//! exactly one provider (the `users.provider` column, immutable). This lane
//! selects between the two providers per call from the user's `provider`
//! column (a typed switch on [`Provider`]), and calls the driver functions
//! directly — so the loop's control flow (the biased select, the backoff, the
//! idle policy, the frame push) is unchanged and provider-agnostic.
//!
//! The unified event type ([`LaneEvent`]) is the loop's `P::Event`: an enum
//! over the two providers' event rows, so the loop's `Select`, the
//! consumed-event channel, and the `run_event` dispatch are all
//! provider-agnostic.
//!
//! The lane is generic over the store type `S` (the transaction's connection)
//! and its entity-op facade `F` ([`EntityFacade`] — the provider's door into
//! the domain): `type Store = S`, so the loop's transaction connection ties
//! to `S`, and the lane's ops forward domain work (the per-call provider
//! selection, the driver domain calls) through `F`. `S` must be the local
//! data store for both providers (`StravaDataStore + IcuDataStore`) — the
//! lane bounds on no full domain store.

use tb_domain::{ActivityId, PartId, Session, Summary, TbResult, UserId, UserStat};

use crate::EntityFacade;
use crate::Provider;
use crate::icu::{IcuDataStore, event::IcuEvent};
use crate::strava::{StravaDataStore, event::Event as StravaEvent};
use crate::{Lane, Probe, ProviderLane};

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

/// The unified provider lane: [`Lane<S, LaneEvent, F>`](Lane) — the
/// [`ProviderLane`] impl the executor loop runs (ADR-0014, amended: the seam's
/// single impl). It selects between the two providers per call from the
/// user's `provider` column (a typed switch on [`Provider`]) and calls the
/// driver functions directly; the loop is generic over this single type and
/// never names a concrete provider.
///
/// Generic over the store type `S` (the transaction's connection) and the
/// entity-op facade `F` (the provider's door into the domain): `type Store =
/// S`, so the loop's transaction connection ties to `S`; `S` must be the
/// local data store for both providers (`StravaDataStore + IcuDataStore`).
/// The production alias is concrete over the concrete facade
/// ([`EntityFacadeImpl`]).
///
/// [`EntityFacadeImpl`]: crate::EntityFacadeImpl
pub type UnifiedLane<S> = Lane<S, LaneEvent, crate::EntityFacadeImpl>;

impl<S, F> Lane<S, LaneEvent, F>
where
    S: StravaDataStore + IcuDataStore,
    F: EntityFacade<S>,
{
    /// The user's provider, read from the `users.provider` column (through
    /// the entity-op facade's [`EntityFacade::user_provider`] read — the
    /// owning entity's op, ADR-0010) and parsed into the typed [`Provider`]
    /// (the single dispatch mechanism, ADR-0014). A read failure is a DB
    /// failure the caller classifies.
    async fn provider_of(store: &mut S, user_id: UserId, facade: &F) -> TbResult<Provider> {
        let s = facade.user_provider(&user_id, store).await?;
        Provider::try_from(s.as_str())
    }
}

#[async_trait::async_trait]
impl<S, F> ProviderLane for Lane<S, LaneEvent, F>
where
    S: StravaDataStore + IcuDataStore + Send + 'static,
    F: EntityFacade<S> + Clone + Send + Sync + 'static,
{
    type Event = LaneEvent;
    type Store = S;
    type Facade = F;

    fn facade(&self) -> &F {
        &self.facade
    }

    /// Probe the user's provider event queue (one transaction's
    /// housekeeping): read the user's `provider` column (through the facade),
    /// then read that provider's queue. The returned event is wrapped in
    /// [`LaneEvent`].
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<LaneEvent> {
        let provider = match Self::provider_of(store, user_id, &self.facade).await {
            Ok(provider) => provider,
            Err(_) => return Probe::Error,
        };
        match provider {
            Provider::Icu => match crate::icu::event::get_event(user_id, store).await {
                Ok(crate::icu::event::QueueRead::Event(event)) => {
                    Probe::Event(LaneEvent::Icu(event))
                }
                Ok(crate::icu::event::QueueRead::Limited { until }) => Probe::Limited(until),
                Ok(crate::icu::event::QueueRead::Empty) => Probe::Empty,
                Err(_) => Probe::Error,
            },
            Provider::Strava => match crate::strava::event::get_event(user_id, store).await {
                Ok(crate::strava::event::QueueRead::Event(event)) => {
                    Probe::Event(LaneEvent::Strava(event))
                }
                Ok(crate::strava::event::QueueRead::Limited { until }) => Probe::Limited(until),
                Ok(crate::strava::event::QueueRead::Empty) => Probe::Empty,
                Err(_) => Probe::Error,
            },
        }
    }

    /// Process the user's next queued event (the work transaction): read the
    /// user's `provider` column (through the facade), then handle the event
    /// with that provider's driver.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        let provider = match Self::provider_of(store, user_id, &self.facade).await {
            Ok(provider) => provider,
            Err(_) => return None,
        };
        match provider {
            Provider::Icu => crate::icu::event::process(user_id, &self.facade, store)
                .await
                .ok(),
            Provider::Strava => crate::strava::event::process(user_id, &self.facade, store)
                .await
                .ok(),
        }
    }

    /// Queue a sync event on the user's provider event queue (the onboarding
    /// cutover's provider side): the Strava arm queues the `strava_events`
    /// row; the icu arm runs the icu onboarding (gear import + backfill)
    /// through its driver.
    async fn insert_sync(
        &self,
        user_id: UserId,
        time: i64,
        auto: bool,
        store: &mut S,
    ) -> TbResult<()> {
        let provider = Self::provider_of(store, user_id, &self.facade).await?;
        match provider {
            Provider::Icu => crate::icu::onboard(user_id, time, &self.facade, store).await,
            Provider::Strava => crate::strava::event::insert_sync(user_id, time, auto, store).await,
        }
    }

    /// Import the user's gear from their provider (the hydration write's
    /// provider side): the user's provider's driver.
    async fn update_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        let provider = Self::provider_of(store, user_id, &self.facade).await?;
        match provider {
            Provider::Icu => crate::icu::gear::update_gear(user_id, &self.facade, store).await,
            Provider::Strava => {
                crate::strava::StravaUser::update_gear(user_id, &self.facade, store).await
            }
        }
    }

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side): the union of the Strava users' stats and the icu users' stats
    /// (each user's stat is sourced from their own provider).
    async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>> {
        let mut stats = crate::strava::get_all_stats(&self.facade, store).await?;
        stats.extend(crate::icu::get_all_stats(&self.facade, store).await?);
        Ok(stats)
    }

    /// The per-user half of [`ProviderLane::activity_url`]: read the user's
    /// `provider` column (through the facade), then build that provider's URL
    /// (the driver extracts the activity id's prefix).
    async fn activity_url(
        &self,
        id: ActivityId,
        session: Box<dyn Session>,
        store: &mut S,
    ) -> TbResult<String> {
        let provider = Self::provider_of(store, session.user_id(), &self.facade).await?;
        match provider {
            Provider::Icu => {
                crate::icu::activity::activity_url(id, session, &self.facade, store).await
            }
            Provider::Strava => {
                crate::strava::activity::strava_url(id, session, &self.facade, store).await
            }
        }
    }
}
