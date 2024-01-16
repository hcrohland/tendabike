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

//! This module contains the tb_domain logic for parts in the Tendabike system.
//!
//! A `Part` represents a single part of a bike, such as a wheel or a chain. Each part has a unique
//! ID, an owner, a type, a name, and various other attributes that describe its usage and history.
//!
//! The `Assembly` type is a collection of parts that make up a complete bike. It is represented as
//! a `HashMap` of `PartId` keys and `Part` values.
//!
//! This module also defines the `ATrait` trait, which provides a method for looking up a part by ID
//! in an `Assembly`.
//!
//! Finally, this module defines the `NewPart` type, which is used to create new parts in the database.

use crate::traits::Store;

use super::*;
use ::time::OffsetDateTime;
use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};

/// The database's representation of a part.
#[serde_as]
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Queryable,
    Identifiable,
    Associations,
    AsChangeset,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = schema::parts)]
#[diesel(belongs_to(PartType, foreign_key = what))]
pub struct Part {
    /// The primary key
    pub id: PartId,
    /// The owner
    pub owner: UserId,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    /// purchase date
    #[serde_as(as = "Rfc3339")]
    pub purchase: OffsetDateTime,
    /// usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
    /// usage count
    pub count: i32,
    /// last time it was used
    #[serde_as(as = "Rfc3339")]
    pub last_used: OffsetDateTime,
    /// Was it disposed? If yes, when?
    #[serde_as(as = "Option<Rfc3339>")]
    pub disposed_at: Option<OffsetDateTime>,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Insertable)]
#[diesel(table_name = schema::parts)]
pub struct NewPart {
    /// The owner
    pub owner: UserId,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    #[serde_as(as = "Option<Rfc3339>")]
    pub purchase: Option<OffsetDateTime>,
}

use serde_with::serde_as;
use time::format_description::well_known::Rfc3339;
#[serde_as]
#[derive(Clone, Debug, PartialEq, Deserialize, AsChangeset)]
#[diesel(table_name = schema::parts)]
#[diesel(treat_none_as_null = true)]
pub struct ChangePart {
    pub id: PartId,
    /// The owner
    pub owner: UserId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    #[serde_as(as = "Rfc3339")]
    pub purchase: OffsetDateTime,
    /// Was it disposed? If yes, when?
    #[serde_as(as = "Option<Rfc3339>")]
    pub disposed_at: Option<OffsetDateTime>,
}

#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

impl PartId {
    pub fn new(id: i32) -> PartId {
        PartId(id)
    }

    pub async fn get(id: i32, user: &dyn Person, conn: &mut impl Store) -> TbResult<PartId> {
        PartId(id).checkuser(user, conn).await
    }

    /// get the part with id part
    pub async fn part(self, user: &dyn Person, conn: &mut impl Store) -> TbResult<Part> {
        let part = conn.partid_get_part(self).await?;
        user.check_owner(
            part.owner,
            format!("user {} cannot access part {}", user.get_id(), part.id),
        )?;
        Ok(part)
    }

    /// get the name of the part
    ///
    /// does not check ownership. This is needed for rentals.
    pub async fn name(self, conn: &mut impl Store) -> TbResult<String> {
        conn.partid_get_name(self).await
    }

    pub async fn what(self, conn: &mut impl Store) -> TbResult<PartTypeId> {
        conn.partid_get_type(self).await
    }

    /// check if the given user is the owner or an admin.
    /// Returns Forbidden if not.
    pub async fn checkuser(self, user: &dyn Person, conn: &mut impl Store) -> TbResult<PartId> {
        let own = conn.partid_get_ownerid(self, user).await?;
        if user.get_id() == own {
            return Ok(self);
        }

        Err(crate::Error::NotFound(format!(
            "user {} cannot access part {}",
            user.get_id(),
            self
        )))
    }

    /// apply a usage to the part with given id
    ///
    /// If the stored purchase date is later than the usage date, it will adjust the purchase date
    /// returns the changed part
    pub async fn apply_usage(
        self,
        usage: &Usage,
        start: OffsetDateTime,
        conn: &mut impl Store,
    ) -> TbResult<Part> {
        trace!("Applying usage {:?} to part {}", usage, self);
        conn.partid_apply_usage(self, usage, start).await
    }
}

impl Part {
    /// Returns a list of all parts owned by the given user.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to a `dyn Person` trait object representing the user.
    /// * `conn` - A mutable reference to an `AppConn` object representing the database connection.
    ///
    /// # Returns
    ///
    /// A `Vec` of `Part` objects owned by the given user.
    ///
    /// # Errors
    ///
    /// Returns an `TbResult` object that may contain a `diesel::result::Error` if the query fails.
    pub async fn get_all(user: &UserId, conn: &mut impl Store) -> TbResult<Vec<Part>> {
        conn.part_get_all_for_userid(user).await
    }

    /// reset all usage counters for all parts of a person
    ///
    /// returns the list of main gears affected
    pub async fn reset(user: &dyn Person, conn: &mut impl Store) -> TbResult<Vec<PartId>> {
        use std::collections::HashSet;

        // reset all counters for all parts of this user
        let part_list = conn.parts_reset_all_usages(user.get_id()).await?;

        // get the main types
        let mains: HashSet<PartTypeId> = conn.parttypes_all_maingear().await?.into_iter().collect();

        // only return the main parts
        Ok(part_list
            .into_iter()
            .filter(|x| mains.contains(&x.what))
            .map(|x| x.id)
            .collect())
    }
}

impl NewPart {
    pub async fn create(self, user: &dyn Person, conn: &mut impl Store) -> TbResult<Part> {
        info!("Create {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        let now = OffsetDateTime::now_utc();
        let createtime = self.purchase.unwrap_or(now);
        conn.create_part(self, createtime).await
    }
}

impl ChangePart {
    pub async fn change(self, user: &dyn Person, conn: &mut impl Store) -> TbResult<Part> {
        info!("Change {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        conn.part_change(self).await
    }
}

impl PartDomain for Domain {}
