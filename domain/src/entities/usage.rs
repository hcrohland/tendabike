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

use super::*;
use crate::schema::*;
use crate::UsageStore;
use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use std::ops::{Add, Neg, Sub};
use uuid::Uuid;

#[derive(
    DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub struct UsageId(Uuid);

NewtypeDisplay! { () pub struct UsageId(); }
NewtypeFrom! { () pub struct UsageId(Uuid); }

#[derive(
    Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable, AsChangeset,
)]
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
    /// Overall descending
    pub power: i32,
    /// number of activities
    pub count: i32,
}

impl Usage {
    pub(crate) async fn create(store: &mut impl UsageStore) -> TbResult<Usage> {
        let res = Usage {
            id: Uuid::now_v7().into(),
            ..Default::default()
        };
        return store.create(&res).await.map(|_| res);
    }

    pub(crate) async fn update_vec(
        vec: Vec<&Self>,
        store: &mut impl UsageStore,
    ) -> TbResult<usize> {
        return store.update(vec).await;
    }
}

impl UsageId {
    pub(crate) async fn read(self, store: &mut impl UsageStore) -> TbResult<Usage> {
        return store.read(self).await;
    }
}

impl Add for Usage {
    type Output = Self;
    /// Add an activity to of a usage
    ///
    /// If the descend value is missing, assume descend = climb
    fn add(self, rhs: Self) -> Self {
        Usage {
            id: self.id,
            time: self.time + rhs.time,
            climb: self.climb + rhs.climb,
            descend: self.descend + rhs.descend,
            power: self.power + rhs.power,
            distance: self.distance + rhs.distance,
            count: self.count + rhs.count,
        }
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
            power: self.power - rhs.power,
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
            power: -self.power,
            distance: -self.distance,
            count: -self.count,
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use crate::{Error, TbResult, Usage, UsageId, UsageStore};

    struct MemStore(std::collections::HashMap<UsageId, Usage>);

    #[async_session::async_trait]
    impl UsageStore for MemStore {
        async fn read(&mut self, uid: UsageId) -> TbResult<Usage> {
            self.0
                .get(&uid)
                .ok_or(Error::NotFound(format!("UsageId {uid} not found")))
                .map(|u| u.clone())
        }

        async fn create(&mut self, usage: &Usage) -> TbResult<usize> {
            if self.0.contains_key(&usage.id) {
                return Err(Error::Conflict(format!(
                    "UsageId {} does already exist",
                    usage.id
                )));
            }

            self.0.insert(usage.id, usage.clone());
            return Ok(1);
        }

        async fn update(&mut self, vec: Vec<&Usage>) -> TbResult<usize> {
            for usage in &vec {
                if self.0.insert(usage.id, (*usage).clone()) == None {
                    return Err(Error::Conflict(format!(
                        "UsageId {} does not exist",
                        usage.id
                    )));
                }
            }
            return Ok(vec.len());
        }
    }

    #[tokio::test]
    async fn create_usage_returns() -> TbResult<()> {
        let mut store = MemStore(HashMap::new());
        let usage = Usage::create(&mut store).await?;
        assert_eq!(usage.climb, 0);
        let usage2 = Usage {
            count: 1,
            climb: 2,
            descend: 3,
            ..Default::default()
        };
        let usage3 = usage.clone() + usage2.clone() + usage2;
        assert_eq!((&usage3).climb, 4);
        assert_eq!((&usage3).count, 2);
        assert_eq!((&usage3).descend, 6);
        assert_eq!((&usage3).time, 0);
        Usage::update_vec(vec![&usage3], &mut store).await?;
        let usage4 = usage3.id.read(&mut store).await?;
        assert_eq!(usage3, usage4);
        assert_eq!(usage4 - usage3, usage);
        Ok(())
    }
}
