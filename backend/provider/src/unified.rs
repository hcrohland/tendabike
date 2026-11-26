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

//! The provider lane (spec #1029, ticket #1033): the [`ProviderLane`] impl
//! the executor loop runs in production.
//!
//! The per-provider lane is generic over the provider marker
//! [`ProviderSpec`] and the [`Provider`](crate::driver::Provider) driver: it
//! is bounded on the single [`Store`](crate::store::Store) interface and
//! drives the `Provider` trait methods, so it is provider-agnostic.
//!
//! The **unified lane** ([`UnifiedLane`], ADR-0014) is the seam's single
//! impl: it wraps the two per-provider lanes (one per provider marker +
//! driver) and implements `probe`/`process`/`import_gear`/`queue_sync` by
//! doing the [`provider_of`](crate::provider_of) lookup + a typed switch on
//! the [`Provider`](crate::Provider) enum, delegating to the appropriate
//! per-provider lane. Its event type is the [`UnifiedEvent`] enum (one
//! variant per provider), consumed opaquely by the loop. A third provider
//! costs a variant in [`UnifiedEvent`] and an arm in the unified lane's
//! switch — `tb_exec` is untouched.
//!
//! The lane is generic over the store type `S` (the transaction's connection)
//! and its entity-op facade `F` ([`EntityFacade`] — the provider's door into
//! the domain): `type Store = S`, so the loop's transaction connection ties
//! to `S`, and the lane's ops forward domain work through `F`. `S` must
//! implement [`Store<P>`] for the lane's provider marker `P` — the lane
//! bounds on no per-provider store-trait intersection.

use crate::ProviderSpec;
use crate::driver::Provider;
use crate::facade::{Error, PartId, Summary, TbResult, UserId, UserStat};
use crate::store::Store;

use crate::EntityFacade;
use crate::{Lane, Probe, ProviderLane};

/// The provider lane: [`Lane<P, S, F, D>`](Lane) — the per-provider
/// [`ProviderLane`] impl. It is generic over the provider marker `P`, the
/// store `S`, the entity-op facade `F`, and the driver `D`: it is bounded on
/// the single [`Store<P>`] interface and drives the [`Provider<P>`] trait
/// methods, so it is provider-agnostic. The seam's single impl is the
/// unified lane ([`UnifiedLane`], ADR-0014), which resolves the user's
/// provider internally, in `probe`, and delegates to a per-provider lane
/// like this one; the registry and the loop never name a concrete provider.
///
/// The production alias is concrete over the concrete facade
/// ([`EntityFacadeImpl`]).
///
/// [`EntityFacadeImpl`]: crate::EntityFacadeImpl
pub type ProviderLaneImpl<P, S, D> = Lane<P, S, crate::EntityFacadeImpl, D>;

/// The unified event type (ADR-0014): an enum over the per-provider event
/// types (one variant per provider). The unified lane's `type Event` — the
/// loop consumes it opaquely (it calls `process` after a `Probe::Event` and
/// never inspects the event's contents), so the variants' payloads do not
/// leak into the loop.
#[derive(Debug, Clone, PartialEq)]
pub enum UnifiedEvent {
    /// A Strava event.
    Strava(crate::strava::event::Event),
    /// An icu event.
    Icu(crate::icu::IcuEvent),
}

/// The unified provider lane (ADR-0014): the [`ProviderLane`] impl the
/// executor loop runs in production. It wraps the two per-provider lanes
/// (one per provider marker + driver) and implements `probe`/`process`/
/// `import_gear`/`queue_sync` by doing the
/// [`provider_of`](crate::provider_of) lookup + a typed switch on the
/// [`Provider`](crate::Provider) enum, delegating to the appropriate
/// per-provider lane. It is the seam's single impl (ADR-0014): a third
/// provider costs a variant in the [`UnifiedEvent`] enum and an arm in this
/// lane's switch — `tb_exec` is untouched.
///
/// The lane is generic over the store type `S` (the transaction's connection)
/// and its entity-op facade `F` ([`EntityFacade`] — the provider's door into
/// the domain): `type Store = S`, so the loop's transaction connection ties
/// to `S`, and the lane's ops forward domain work through `F`. `S` must
/// implement [`Store<P>`] for both provider markers (the lane's
/// `provider_of` lookup reads both providers' user tables).
pub struct UnifiedLane<S, F, D1, D2> {
    strava_lane: Lane<crate::strava::StravaSpec, S, F, D1>,
    icu_lane: Lane<crate::icu::IcuSpec, S, F, D2>,
}

impl<S, F, D1, D2> UnifiedLane<S, F, D1, D2> {
    /// The unified lane over the two per-provider lanes.
    pub fn new(
        strava_lane: Lane<crate::strava::StravaSpec, S, F, D1>,
        icu_lane: Lane<crate::icu::IcuSpec, S, F, D2>,
    ) -> Self {
        Self {
            strava_lane,
            icu_lane,
        }
    }

    /// The admin all-user stat read (the `ServiceApi`'s `all_stats`): every
    /// enabled provider's users as provider-neutral [`UserStat`]s — the two
    /// lanes are driven in one transaction and the results concatenated, in
    /// the `Provider` enum's definition order. A disabled provider's lane is
    /// skipped (ADR-0016: its users are frozen on this server, not listed).
    pub async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>>
    where
        S: crate::ProviderStore + 'static,
        F: EntityFacade<S> + Clone + Send + Sync + 'static,
        D1: Provider<crate::strava::StravaSpec> + Send + Sync + 'static,
        D2: Provider<crate::icu::IcuSpec> + Send + Sync + 'static,
    {
        let mut stats = Vec::new();
        if crate::enabled_providers().contains(&crate::Provider::Strava) {
            stats.extend(D1::all_stats(self.facade(), store).await?);
        }
        if crate::enabled_providers().contains(&crate::Provider::Icu) {
            stats.extend(D2::all_stats(self.facade(), store).await?);
        }
        Ok(stats)
    }
}

impl<S, F, D1, D2> Clone for UnifiedLane<S, F, D1, D2>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            strava_lane: self.strava_lane.clone(),
            icu_lane: self.icu_lane.clone(),
        }
    }
}

impl<S, F, D1, D2> std::fmt::Debug for UnifiedLane<S, F, D1, D2> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifiedLane").finish()
    }
}

#[async_trait::async_trait]
impl<S, F, D1, D2> ProviderLane for UnifiedLane<S, F, D1, D2>
where
    S: crate::ProviderStore + 'static,
    F: EntityFacade<S> + Clone + Send + Sync + 'static,
    D1: Provider<crate::strava::StravaSpec> + Send + Sync + 'static,
    D2: Provider<crate::icu::IcuSpec> + Send + Sync + 'static,
{
    type Event = UnifiedEvent;
    type Store = S;
    type Facade = F;

    fn facade(&self) -> &F {
        &self.strava_lane.facade
    }

    /// Probe the user's provider event queue: resolve the user's provider
    /// (the [`provider_of`](crate::provider_of) lookup) and delegate to the
    /// user's own (marker, driver) lane, wrapping the event in the
    /// [`UnifiedEvent`] enum. The queue's read-side rules (the rate-limit
    /// gate, the latest-per-key dedup) are the per-provider lane's — the
    /// unified lane only selects the lane.
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<UnifiedEvent> {
        let provider = match crate::provider_of(store, user_id).await {
            Ok(provider) => provider,
            Err(Error::NotFound(_)) => return Probe::UserGone,
            Err(_) => return Probe::Error,
        };
        match provider {
            crate::Provider::Strava => match self.strava_lane.probe(user_id, store).await {
                Probe::Event(e) => Probe::Event(UnifiedEvent::Strava(e)),
                Probe::Limited(until) => Probe::Limited(until),
                Probe::Empty => Probe::Empty,
                Probe::UserGone => Probe::UserGone,
                Probe::Error => Probe::Error,
            },
            crate::Provider::Icu => match self.icu_lane.probe(user_id, store).await {
                Probe::Event(e) => Probe::Event(UnifiedEvent::Icu(e)),
                Probe::Limited(until) => Probe::Limited(until),
                Probe::Empty => Probe::Empty,
                Probe::UserGone => Probe::UserGone,
                Probe::Error => Probe::Error,
            },
        }
    }

    /// Process the user's next queued event (the work transaction): resolve
    /// the user's provider and delegate to the user's own (marker, driver)
    /// lane. The lane's `process` classifies the provider-side failures
    /// (the rate-limit `Stop`, the disable, the delete) — the unified lane
    /// only selects the lane.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        let provider = match crate::provider_of(store, user_id).await {
            Ok(provider) => provider,
            Err(_) => return None,
        };
        match provider {
            crate::Provider::Strava => self.strava_lane.process(user_id, store).await,
            crate::Provider::Icu => self.icu_lane.process(user_id, store).await,
        }
    }

    /// Import the user's gear (bikes + shoes) from the provider (the
    /// onboarding's first step, spec #829): resolve the user's provider and
    /// delegate to the user's own (marker, driver) lane.
    async fn import_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        let provider = crate::provider_of(store, user_id).await?;
        match provider {
            crate::Provider::Strava => self.strava_lane.import_gear(user_id, store).await,
            crate::Provider::Icu => self.icu_lane.import_gear(user_id, store).await,
        }
    }

    /// Queue the activity sync on the provider's event queue (the
    /// onboarding's second step, spec #829): resolve the user's provider and
    /// delegate to the user's own (marker, driver) lane.
    async fn queue_sync(&self, user_id: UserId, time: i64, store: &mut S) -> TbResult<()> {
        let provider = crate::provider_of(store, user_id).await?;
        match provider {
            crate::Provider::Strava => self.strava_lane.queue_sync(user_id, time, store).await,
            crate::Provider::Icu => self.icu_lane.queue_sync(user_id, time, store).await,
        }
    }
}

/// The production alias for the unified lane: concrete over the concrete
/// facade ([`EntityFacadeImpl`]) and the two concrete drivers
/// ([`StravaDriver`], [`IcuDriver`]).
///
/// [`StravaDriver`]: crate::strava::driver::StravaDriver
/// [`IcuDriver`]: crate::icu::driver::IcuDriver
pub type UnifiedLaneImpl<S> = UnifiedLane<
    S,
    crate::EntityFacadeImpl,
    crate::strava::driver::StravaDriver,
    crate::icu::driver::IcuDriver,
>;

#[async_trait::async_trait]
impl<P, S, F, D> ProviderLane for Lane<P, S, F, D>
where
    P: ProviderSpec,
    S: Store<P> + Send + 'static,
    F: EntityFacade<S> + Clone + Send + Sync + 'static,
    D: Provider<P> + Send + Sync + 'static,
{
    type Event = P::Event;
    type Store = S;
    type Facade = F;

    fn facade(&self) -> &F {
        &self.facade
    }

    /// Probe the user's provider event queue (one transaction's
    /// housekeeping): resolve the user's provider id (the queue's partition
    /// key) from the provider-internal user data, then read that provider's
    /// queue with the kit's [`get_event`](crate::queue::get_event) — the
    /// queue's read-side rules are provider-agnostic.
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<P::Event> {
        let user = match store.user_get_by_tbid(user_id).await {
            Ok(user) => user,
            Err(Error::NotFound(_)) => return Probe::UserGone,
            Err(_) => return Probe::Error,
        };
        let owner = P::user_owner(&user).clone();
        match crate::queue::get_event::<P, _>(owner, store).await {
            Ok(read) => read.into_probe(std::convert::identity),
            Err(_) => Probe::Error,
        }
    }

    /// Process the user's next queued event (the work transaction): drive the
    /// [`Provider<P>`] driver's `process` method.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        // A disabled provider's queued events are declined, not processed
        // (ADR-0016): the event stays queued (the caller rolls back and
        // backs off), so re-adding the credential restores the provider
        // and its syncs.
        if crate::config::is_disabled(P::PROVIDER) {
            return None;
        }
        D::process(user_id, &self.facade, store).await.ok()
    }

    /// The onboarding's first step (spec #829): import the user's gear
    /// (bikes + shoes) from the provider, returning the created part ids.
    /// Drives the [`Provider<P>`] driver's `import_gear` method.
    async fn import_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        D::import_gear(user_id, &self.facade, store).await
    }

    /// The onboarding's second step (spec #829): queue the activity sync on
    /// the user's provider queue. Drives the [`Provider<P>`] driver's
    /// `insert_sync` method.
    async fn queue_sync(&self, user_id: UserId, time: i64, store: &mut S) -> TbResult<()> {
        D::insert_sync(user_id, time, store).await
    }
}
