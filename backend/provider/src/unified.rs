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
//! The lane is generic over the provider marker [`ProviderSpec`] and the
//! [`Provider`](crate::driver::Provider) driver: it is bounded on the single
//! [`Store`](crate::store::Store) interface and drives the `Provider` trait
//! methods, so it is provider-agnostic. Provider selection happens at the
//! call site (the dispatch), not in the lane.
//!
//! The lane is generic over the store type `S` (the transaction's connection)
//! and its entity-op facade `F` ([`EntityFacade`] — the provider's door into
//! the domain): `type Store = S`, so the loop's transaction connection ties
//! to `S`, and the lane's ops forward domain work through `F`. `S` must
//! implement [`Store<P>`] for the lane's provider marker `P` — the lane
//! bounds on no per-provider store-trait intersection.
//!
//! The lane's event type is `P::Event` (the provider's event row), so the
//! loop's `Select`, the consumed-event channel, and the `run_event` dispatch
//! are all provider-agnostic.

use crate::ProviderSpec;
use crate::driver::Provider;
use crate::facade::{Error, PartId, Summary, TbResult, UserId};
use crate::store::Store;

use crate::EntityFacade;
use crate::{Lane, Probe, ProviderLane};

/// The provider lane: [`Lane<P, S, F, D>`](Lane) — the [`ProviderLane`] impl
/// the executor loop runs. It is generic over the provider marker `P`, the
/// store `S`, the entity-op facade `F`, and the driver `D`: it is bounded on
/// the single [`Store<P>`] interface and drives the [`Provider<P>`] trait
/// methods, so it is provider-agnostic. Provider selection happens at the
/// call site (the dispatch), not in the lane.
///
/// The production alias is concrete over the concrete facade
/// ([`EntityFacadeImpl`]).
///
/// [`EntityFacadeImpl`]: crate::EntityFacadeImpl
pub type ProviderLaneImpl<P, S, D> = Lane<P, S, crate::EntityFacadeImpl, D>;

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
