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

//! This module contains the `Usage` struct and its implementation.
//! The `Usage` struct represents the usage of a part, including time, distance, climbing, descending, power, and count.
//! It also provides methods to add an activity to the usage.

use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::ops::{Add, Neg, Sub};
use uuid::Uuid;

use crate::*;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UsageId(Uuid);

NewtypeDisplay! { () pub struct UsageId(); }
NewtypeFrom! { () pub struct UsageId(Uuid); }

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Usage {
    // id for referencing
    pub id: UsageId,
    // usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
    /// Overall energy
    pub energy: i32,
    /// number of activities
    pub count: i32,
}

impl Usage {
    pub(crate) async fn update(self, store: &mut impl UsageStore) -> TbResult<Usage> {
        Usage::update_vec(&[&self], store).await?;
        Ok(self)
    }

    pub(crate) async fn update_vec<U>(vec: &[U], store: &mut impl UsageStore) -> TbResult<usize>
    where
        U: Borrow<Usage> + Sync,
    {
        store.update(vec).await
    }

    pub(crate) fn new(id: UsageId) -> Usage {
        Usage {
            id,
            ..Default::default()
        }
    }

    pub(crate) async fn delete_all(store: &mut impl UsageStore) -> TbResult<usize> {
        store.delete_all().await
    }

    pub(crate) async fn get_vec(
        vec: &[UsageId],
        store: &mut impl UsageStore,
    ) -> TbResult<Vec<Usage>> {
        let mut res = Vec::new();
        for id in vec {
            res.push(id.read(store).await?);
        }
        Ok(res)
    }
}

impl UsageId {
    pub(crate) fn new() -> Self {
        Uuid::now_v7().into()
    }

    pub(crate) async fn delete(self, store: &mut impl UsageStore) -> TbResult<Usage> {
        match store.delete(self).await {
            Err(Error::NotFound(_)) => Ok(Usage::new(self)),
            x => x,
        }
    }

    pub(crate) async fn read(self, store: &mut impl UsageStore) -> TbResult<Usage> {
        store
            .get(self)
            .await
            .map(|u| u.unwrap_or_else(|| Usage::new(self)))
    }
}

impl<U> Add<U> for &Usage
where
    U: Borrow<Usage>,
{
    type Output = Usage;
    /// Add an activity to of a usage
    ///
    /// If the descend value is missing, assume descend = climb
    fn add(self, rhs: U) -> Usage {
        let rhs = rhs.borrow();
        Usage {
            id: self.id,
            time: self.time + rhs.time,
            climb: self.climb + rhs.climb,
            descend: self.descend + rhs.descend,
            energy: self.energy + rhs.energy,
            distance: self.distance + rhs.distance,
            count: self.count + rhs.count,
        }
    }
}

impl<U> Add<U> for Usage
where
    U: Borrow<Usage>,
{
    type Output = Self;
    /// Add an activity to of a usage
    ///
    /// If the descend value is missing, assume descend = climb
    fn add(self, rhs: U) -> Usage {
        let rhs = rhs.borrow();
        Usage {
            id: self.id,
            time: self.time + rhs.time,
            climb: self.climb + rhs.climb,
            descend: self.descend + rhs.descend,
            energy: self.energy + rhs.energy,
            distance: self.distance + rhs.distance,
            count: self.count + rhs.count,
        }
    }
}

impl Add<&Usage> for Vec<Usage> {
    type Output = Self;

    fn add(self, rhs: &Usage) -> Self {
        self.into_iter().map(|u| u + rhs).collect()
    }
}

impl Sub for Usage {
    type Output = Self;
    /// Add an activity to of a usage
    ///
    /// If the descend value is missing, assume descend = climb
    fn sub(self, rhs: Self) -> Self {
        Usage {
            id: self.id,
            time: self.time - rhs.time,
            climb: self.climb - rhs.climb,
            descend: self.descend - rhs.descend,
            energy: self.energy - rhs.energy,
            distance: self.distance - rhs.distance,
            count: self.count - rhs.count,
        }
    }
}

impl Neg for Usage {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Usage {
            id: self.id,
            time: -self.time,
            climb: -self.climb,
            descend: -self.descend,
            energy: -self.energy,
            distance: -self.distance,
            count: -self.count,
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{borrow::Borrow, collections::HashMap};

    use crate::{TbResult, Usage, UsageId, UsageStore};

    struct MemStore(std::collections::HashMap<UsageId, Usage>);

    #[async_session::async_trait]
    impl UsageStore for MemStore {
        async fn get(&mut self, id: UsageId) -> TbResult<Option<Usage>> {
            Ok(self.0.get(&id).cloned())
        }

        async fn update<U>(&mut self, vec: &[U]) -> TbResult<usize>
        where
            U: Borrow<Usage> + Sync,
        {
            for usage in vec {
                let usage = usage.borrow();
                self.0.insert(usage.id, usage.clone());
            }
            Ok(vec.len())
        }

        async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
            match self.0.remove(&usage) {
                Some(x) => Ok(x),
                None => Err(crate::Error::NotFound(format!("Usage {} not found", usage))),
            }
        }

        async fn delete_all(&mut self) -> TbResult<usize> {
            let res = self.0.len();
            self.0.clear();
            Ok(res)
        }
    }

    #[tokio::test]
    async fn create_usage_returns() -> TbResult<()> {
        let mut store = MemStore(HashMap::new());
        let store = &mut store;
        let usage = UsageId::new().read(store).await?;
        assert_eq!(usage.climb, 0);
        let usage2 = Usage {
            id: UsageId::new(),
            count: 1,
            climb: 2,
            descend: 3,
            ..Default::default()
        };
        let usage3 = &usage + &usage2 + &usage2;
        assert_eq!((&usage3).climb, 4);
        assert_eq!((&usage3).count, 2);
        assert_eq!((&usage3).descend, 6);
        assert_eq!((&usage3).time, 0);
        let usage3 = usage3.update(store).await?;
        let usage4 = usage3.id.read(store).await?;
        assert_eq!(usage3, usage4);
        assert_eq!(usage4 - usage3, usage);
        Usage::delete_all(store).await?;
        assert_eq!(Usage::new(usage2.id), usage2.id.read(store).await?);
        Ok(())
    }
}
