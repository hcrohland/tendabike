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

use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::*;

/// The database's representation of a part.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Queryable, Identifiable, AsChangeset)]
#[diesel(primary_key(id))]
#[diesel(table_name = schema::parts)]
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
    /// last time it was used
    #[serde_as(as = "Rfc3339")]
    pub last_used: OffsetDateTime,
    /// Was it disposed? If yes, when?
    #[serde_as(as = "Option<Rfc3339>")]
    pub disposed_at: Option<OffsetDateTime>,
    /// the usage tracker
    usage: UsageId,
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

    pub async fn get(id: i32, user: &dyn Person, store: &mut impl PartStore) -> TbResult<PartId> {
        PartId(id).checkuser(user, store).await
    }

    pub(crate) async fn read(self, store: &mut impl PartStore) -> TbResult<Part> {
        store.partid_get_part(self).await
    }

    /// get the part with id part
    pub async fn part(self, user: &dyn Person, store: &mut impl PartStore) -> TbResult<Part> {
        let part = self.read(store).await?;
        user.check_owner(
            part.owner,
            format!("user {} cannot access part {}", user.get_id(), part.id),
        )?;
        Ok(part)
    }

    /// get the name of the part
    ///
    /// does not check ownership. This is needed for rentals.
    pub async fn name(self, store: &mut impl PartStore) -> TbResult<String> {
        Ok(self.read(store).await?.name)
    }

    pub async fn what(self, store: &mut impl PartStore) -> TbResult<PartTypeId> {
        Ok(self.read(store).await?.what)
    }

    pub async fn is_main(self, store: &mut impl PartStore) -> TbResult<bool> {
        let part = self.read(store).await?;
        part.what.is_main()
    }

    /// check if the given user is the owner or an admin.
    /// Returns Forbidden if not.
    pub async fn checkuser(
        self,
        user: &dyn Person,
        store: &mut impl PartStore,
    ) -> TbResult<PartId> {
        let own = self.read(store).await?.owner;
        if user.get_id() == own {
            return Ok(self);
        }

        Err(crate::Error::NotFound(format!(
            "user {} cannot access part {}",
            user.get_id(),
            self
        )))
    }

    /// if start is later than last_used update last_used
    pub(crate) async fn update_last_use(
        self,
        start: OffsetDateTime,
        store: &mut impl PartStore,
    ) -> TbResult<Part> {
        let mut part = self.read(store).await?;
        if start > part.last_used {
            part.last_used = start;
            store.part_update(&part).await?;
        }
        Ok(part)
    }
}

impl Part {
    pub(crate) async fn get_all(pid: &UserId, store: &mut impl Store) -> TbResult<Vec<Part>> {
        store.part_get_all_for_userid(pid).await
    }

    /// Returns a list of all parts owned by the given user.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to a `dyn Person` trait object representing the user.
    /// * `store` - A mutable reference to an `AppConn` object representing the database connection.
    ///
    /// # Returns
    ///
    /// A `Vec` of `Part` objects owned by the given user.
    ///
    /// # Errors
    ///
    /// Returns an `TbResult` object that may contain a `diesel::result::Error` if the query fails.
    pub(crate) async fn get_part_summary(
        user: &UserId,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let parts = store.part_get_all_for_userid(user).await?;
        let mut usages = Vec::new();
        let mut attachments = Vec::new();
        let mut services = Vec::new();
        let mut plans = Vec::new();
        for part in &parts {
            usages.push(part.usage().read(store).await?);
            let (mut atts, mut uses) = Attachment::for_part_with_usage(part.id, store).await?;
            usages.append(&mut uses);
            attachments.append(&mut atts);
            let (mut servs, mut uses) = Service::for_part_with_usage(part.id, store).await?;
            let mut splans = ServicePlan::for_part(part.id, store).await?;
            usages.append(&mut uses);
            services.append(&mut servs);
            plans.append(&mut splans)
        }
        Ok(Summary {
            parts,
            usages,
            attachments,
            services,
            plans,
            ..Default::default()
        })
    }

    pub(crate) fn usage(&self) -> UsageId {
        self.usage
    }
}

impl NewPart {
    pub async fn create(self, user: &dyn Person, store: &mut impl PartStore) -> TbResult<Part> {
        info!("Create {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        let now = OffsetDateTime::now_utc();
        let createtime = self.purchase.unwrap_or(now);
        store.part_create(self, createtime, UsageId::new()).await
    }
}

impl ChangePart {
    pub async fn change(self, user: &dyn Person, store: &mut impl PartStore) -> TbResult<Part> {
        info!("Change {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        store.part_change(self).await
    }
}
