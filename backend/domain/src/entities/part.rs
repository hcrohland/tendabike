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

#![allow(clippy::too_many_arguments)]
use std::collections::HashSet;

use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

/// The database's representation of a part.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub usage: UsageId,
    pub source: Option<String>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

impl PartId {
    pub async fn get(id: i32, user: &dyn Person, store: &mut impl PartStore) -> TbResult<PartId> {
        PartId(id).checkuser(user, store).await
    }

    pub async fn delete(self, user: &dyn Person, store: &mut impl Store) -> TbResult<PartId> {
        self.checkuser(user, store).await?;

        let (attachments, _) = Attachment::for_part_with_usage(self, store).await?;
        if !attachments.is_empty() {
            return Err(Error::Conflict("Part is still attached".into()));
        }
        let (services, _) = Service::for_part_with_usage(self, store).await?;
        if !services.is_empty() {
            return Err(Error::Conflict("Part has services logged".into()));
        }

        let plans = ServicePlan::for_part(self, store).await?;
        if !plans.is_empty() {
            return Err(Error::Conflict("Part has active serviceplan".into()));
        }
        store.part_delete(self).await
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
    pub(crate) async fn update_timestamps(
        self,
        start: OffsetDateTime,
        store: &mut impl PartStore,
    ) -> TbResult<Part> {
        let mut part = self.read(store).await?;
        let start = round_time(start);
        if start > part.last_used {
            part.last_used = start;
            part = store.part_update(part).await?;
        }
        if start < part.purchase {
            part.purchase = start;
            part = store.part_update(part).await?;
        }
        Ok(part)
    }

    pub(crate) async fn dispose(
        &self,
        time: OffsetDateTime,
        store: &mut impl Store,
    ) -> Result<Part, Error> {
        debug!("-- disposing part {self} at {time}");
        let mut part = self.read(store).await?;
        part.disposed_at = Some(time);
        store.part_update(part).await
    }

    pub(crate) async fn restore(&self, store: &mut impl Store) -> TbResult<Part> {
        debug!("-- restoring part {self}");
        let mut part = self.read(store).await?;
        part.disposed_at = None;
        store.part_update(part).await
    }

    pub async fn change(
        self,
        name: String,
        vendor: String,
        model: String,
        purchase: OffsetDateTime,
        user: &dyn Person,
        store: &mut impl PartStore,
    ) -> TbResult<Part> {
        info!("Change {self:?}");

        let mut part = self.part(user, store).await?;

        let purchase = round_time(purchase);
        part = Part {
            name,
            vendor,
            model,
            purchase,
            ..part
        };
        store.part_update(part).await
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
        let mut plans = ServicePlan::for_user(user, store).await?;
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

    pub async fn create(
        name: String,
        vendor: String,
        model: String,
        what: PartTypeId,
        source: Option<String>,
        purchase: OffsetDateTime,
        user: &dyn Person,
        store: &mut impl PartStore,
    ) -> TbResult<Part> {
        debug!("Create {name} {vendor} {model}");

        let purchase = round_time(purchase);
        store
            .part_create(
                what,
                name,
                vendor,
                model,
                purchase,
                source,
                UsageId::new(),
                user.get_id(),
            )
            .await
    }

    pub async fn categories(
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<HashSet<PartTypeId>> {
        let parts = store.part_get_all_for_userid(&user.get_id()).await?;
        let mut res = HashSet::new();
        for part in parts {
            if part.what.is_main()? {
                res.insert(part.what);
            }
        }

        Ok(res)
    }
}
