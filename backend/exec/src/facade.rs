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

//! The executor's door into the domain (spec #795).
//!
//! [`DomainFacade`] is the trait the executor goes through to reach the
//! domain: the six dispatch-level operations the executor needs, each a
//! one-line forward to the corresponding `tb_domain` function, generic over
//! the store. The executor is generic over `D: DomainFacade` (static
//! dispatch); the crate's own tests swap in the [`MockDomainFacade`] (a
//! crate-internal `cfg(test)` double).
//!
//! [`MockDomainFacade`]: crate::MockDomainFacade

use tb_domain::{
    Activity, ApiRead, ApiWrite, Part, PartId, ReadOutcome, Session, Shop, ShopId, Store, TbResult,
    UserId, WriteOutcome,
};

/// The executor's door into the domain (spec #795): the six dispatch-level
/// operations the executor needs, each a one-line forward to the corresponding
/// `tb_domain` function, generic over the store.
///
/// Dispatch is static — the executor is generic over `D: DomainFacade`.
/// Under `cfg(test)`, `mockall`'s `#[automock]` generates
/// [`MockDomainFacade`]; the crate's own tests set expectations with the
/// store type as a turbofish, e.g. `expect_part_owner::<MyStore>()`.
///
/// [`MockDomainFacade`]: crate::MockDomainFacade
#[async_trait::async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait DomainFacade<C: Send + Sync>: Send + Sync {
    /// Dispatch an [`ApiWrite`] (the domain's `exec`).
    async fn exec(
        &self,
        write: ApiWrite,
        session: &dyn Session,
        store: &mut C,
    ) -> TbResult<WriteOutcome>;

    /// Dispatch an [`ApiRead`] (the domain's `read`).
    async fn read(
        &self,
        read: ApiRead,
        session: &dyn Session,
        store: &mut C,
    ) -> TbResult<ReadOutcome>;

    /// Resolve a part's owner (the domain's `PartId::owner` — the
    /// session-less read the executor uses to route a shop-context frame).
    async fn part_owner(&self, id: PartId, store: &mut C) -> TbResult<UserId>;

    /// Read a part by id with ownership check (the domain's `PartId::part`).
    async fn part(&self, id: PartId, session: &dyn Session, store: &mut C) -> TbResult<Part>;

    /// Rescan all users' activities (the domain's `Activity::rescan_all`):
    /// the all-users maintenance op the admin rescan drives.
    async fn rescan_all(&self, store: &mut C) -> TbResult<()>;

    /// Check the user is the shop's owner (the domain's `ShopId::check_owner`
    /// — a non-owner is `Forbidden`).
    async fn check_owner(&self, id: ShopId, user: UserId, store: &mut C) -> TbResult<Shop>;
}

/// The concrete [`DomainFacade`]: a stateless unit struct that forwards each
/// operation to the corresponding `tb_domain` function.
#[derive(Debug, Default, Clone, Copy)]
pub struct DomainFacadeImpl;

#[async_trait::async_trait]
impl<C: Store + Sync + 'static> DomainFacade<C> for DomainFacadeImpl {
    async fn exec(
        &self,
        write: ApiWrite,
        session: &dyn Session,
        store: &mut C,
    ) -> TbResult<WriteOutcome> {
        tb_domain::exec(write, session, store).await
    }

    async fn read(
        &self,
        read: ApiRead,
        session: &dyn Session,
        store: &mut C,
    ) -> TbResult<ReadOutcome> {
        tb_domain::read(read, session, store).await
    }

    async fn part_owner(&self, id: PartId, store: &mut C) -> TbResult<UserId> {
        id.owner(store).await
    }

    async fn part(&self, id: PartId, session: &dyn Session, store: &mut C) -> TbResult<Part> {
        id.part(session, store).await
    }

    async fn rescan_all(&self, store: &mut C) -> TbResult<()> {
        Activity::rescan_all(store).await
    }

    async fn check_owner(&self, id: ShopId, user: UserId, store: &mut C) -> TbResult<Shop> {
        id.check_owner(user, store).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The witness test's store type: a unit struct, not the domain store —
    /// the mock serves the canned outcome without touching any store, so the
    /// store type only carries the `DomainFacade<C>` parameter.
    struct LocalStore;

    /// The `#[automock]` witness is the seam the executor's tests mock: set
    /// an expectation and the canned outcome is served without touching the
    /// domain. The store type `C` is inferred from the argument.
    #[tokio::test]
    async fn mock_domain_facade_serves_canned_outcome() -> TbResult<()> {
        let mut mock = MockDomainFacade::new();
        mock.expect_part_owner()
            .returning(|_id, _store| Box::pin(async { Ok(UserId::from(99)) }));
        let mut store = LocalStore;
        let result = mock.part_owner(PartId::from(1), &mut store).await?;
        assert_eq!(result, UserId::from(99));
        Ok(())
    }
}
