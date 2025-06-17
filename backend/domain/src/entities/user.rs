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

//! This module contains the tb_domain logic for the `User` entity.
//!
//! The `User` entity represents a user of the `tendabike` application.
//! It contains information such as the user's name, whether they are an admin, and their activity and parts statistics.
//!
//! The `UserId` type is a newtype wrapper around an `i32` and is used to represent the unique identifier of a `User`.
//!
//! The `Stat` struct is used to represent the statistics of a `User`, including the number of parts and activities associated with the user.
//!
//! The `User` struct contains the fields of a user, including their `id`, `name`, `firstname`, and `is_admin` status.
//!
//! The `Person` trait is implemented for the `User` struct and provides methods for getting the user's `id` and `is_admin` status.
//!
//! The `create`, `update`, `read`, and `get_stat` methods are implemented for the `UserId` type and provide CRUD functionality for `User` entities.

use diesel_derive_newtype::DieselNewType;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};

use crate::*;

#[derive(
    DieselNewType, Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct UserId(i32);

impl UserId {
    pub fn inner(&self) -> i32 {
        self.0
    }
}
NewtypeDisplay! { () pub struct UserId(); }
NewtypeFrom! { () pub struct UserId(i32); }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub firstname: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize)]
pub struct Stat {
    pub user: User,
    parts: i64,
    activities: i64,
}

impl UserId {
    pub async fn read(self, store: &mut impl UserStore) -> TbResult<User> {
        store.get(self).await
    }

    pub async fn get_stat(&self, store: &mut impl Store) -> TbResult<Stat> {
        let user = self.read(store).await?;
        let parts = Part::get_all(self, store).await?.len() as i64;
        let activities = Activity::get_all(self, store).await?.len() as i64;
        Ok(Stat {
            user,
            parts,
            activities,
        })
    }

    pub async fn create(
        firstname: &str,
        lastname: &str,
        store: &mut impl UserStore,
    ) -> TbResult<Self> {
        store.create(firstname, lastname).await.map(|u| u.id)
    }

    pub async fn update(
        &self,
        firstname_: &str,
        lastname: &str,
        store: &mut impl UserStore,
    ) -> TbResult<Self> {
        store.update(self, firstname_, lastname).await.map(|u| u.id)
    }

    pub async fn is_admin(&self, store: &mut impl UserStore) -> TbResult<bool> {
        self.read(store).await.map(|u| u.is_admin)
    }

    /// get all parts, attachments and activities for the user
    pub async fn get_summary(&self, store: &mut impl Store) -> TbResult<Summary> {
        use crate::*;
        let activities = Activity::get_all(self, store).await?;
        let summary = Part::get_part_summary(self, store).await?;
        Ok(Summary {
            activities,
            ..summary
        })
    }
}

impl Person for User {
    fn get_id(&self) -> UserId {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}
