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

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::ops::{Add, Neg, Sub};
use uuid::Uuid;

use crate::*;

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    From,
    Into,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Default,
)]
pub struct UsageId(Uuid);

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

    use crate::{TbResult, Usage, UsageId, test_support::MemStore};

    /// create_usage_returns creates and reads usage correctly
    #[tokio::test]
    async fn create_usage_returns() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
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
        assert_eq!(usage3.climb, 4);
        assert_eq!(usage3.count, 2);
        assert_eq!(usage3.descend, 6);
        assert_eq!(usage3.time, 0);
        let usage3 = usage3.update(store).await?;
        let usage4 = usage3.id.read(store).await?;
        assert_eq!(usage3, usage4);
        assert_eq!(usage4 - usage3, usage);
        Usage::delete_all(store).await?;
        assert_eq!(Usage::new(usage2.id), usage2.id.read(store).await?);
        Ok(())
    }

    /// Usage::add combines two usages correctly
    #[test]
    fn usage_add_two_usages() {
        let u1 = Usage {
            id: UsageId::default(),
            time: 1000,
            distance: 5000,
            climb: 100,
            descend: 80,
            energy: 500,
            count: 1,
        };
        let u2 = Usage {
            id: UsageId::default(),
            time: 2000,
            distance: 15000,
            climb: 300,
            descend: 200,
            energy: 1500,
            count: 2,
        };

        let combined = &u1 + &u2;
        assert_eq!(combined.time, 3000);
        assert_eq!(combined.distance, 20000);
        assert_eq!(combined.climb, 400);
        assert_eq!(combined.descend, 280);
        assert_eq!(combined.energy, 2000);
        assert_eq!(combined.count, 3);
    }

    /// Usage::sub correctly subtracts two usages
    #[test]
    fn usage_subtracts_correctly() {
        let u1 = Usage {
            id: UsageId::default(),
            time: 3000,
            distance: 20000,
            climb: 400,
            descend: 280,
            energy: 2000,
            count: 3,
        };
        let u2 = Usage {
            id: UsageId::default(),
            time: 1000,
            distance: 5000,
            climb: 100,
            descend: 80,
            energy: 500,
            count: 1,
        };

        let diff = u1 - u2;
        assert_eq!(diff.time, 2000);
        assert_eq!(diff.distance, 15000);
        assert_eq!(diff.climb, 300);
        assert_eq!(diff.descend, 200);
        assert_eq!(diff.energy, 1500);
        assert_eq!(diff.count, 2);
    }

    /// Usage::neg produces fully inverted usage
    #[test]
    fn usage_negation_produces_inverted() {
        let u = Usage {
            id: UsageId::default(),
            time: 1000,
            distance: 5000,
            climb: 100,
            descend: 80,
            energy: 500,
            count: 1,
        };

        let neg_u = -u;
        assert_eq!(neg_u.time, -1000);
        assert_eq!(neg_u.distance, -5000);
        assert_eq!(neg_u.climb, -100);
        assert_eq!(neg_u.descend, -80);
        assert_eq!(neg_u.energy, -500);
        assert_eq!(neg_u.count, -1);
    }

    /// Usage add_vec adds a single usage to all elements
    #[test]
    fn usage_add_vec_adds_single() {
        let u1 = Usage {
            id: UsageId::default(),
            time: 1000,
            distance: 5000,
            climb: 100,
            descend: 80,
            energy: 500,
            count: 1,
        };
        let u2 = Usage {
            id: UsageId::default(),
            time: 2000,
            distance: 15000,
            climb: 300,
            descend: 200,
            energy: 1500,
            count: 2,
        };
        let increment = Usage {
            id: UsageId::default(),
            time: 500,
            distance: 2000,
            climb: 100,
            descend: 50,
            energy: 250,
            count: 1,
        };

        let result = vec![u1, u2] + &increment;
        assert_eq!(result[0].time, 1500);
        assert_eq!(result[1].time, 2500);
        assert_eq!(result[0].distance, 7000);
        assert_eq!(result[1].distance, 17000);
    }
}
