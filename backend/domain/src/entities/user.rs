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

use anyhow::Context;
use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};

use crate::*;

#[derive(
    Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize, From, Into, Display,
)]
pub struct UserId(i32);

/// Onboarding status enum for tracking user setup progress
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(i32)]
pub enum OnboardingStatus {
    /// User has not completed initial activity sync
    #[default]
    Pending = 0,
    /// User chose to postpone initial activity sync
    InitialSyncPostponed = 2,
    /// User has completed onboarding
    Completed = 99,
}

impl std::convert::TryFrom<i32> for OnboardingStatus {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Pending),
            2 => Ok(Self::InitialSyncPostponed),
            99 => Ok(Self::Completed),
            _ => Err(Error::BadRequest(format!(
                "Invalid onboarding status: {}",
                value
            ))),
        }
    }
}

impl From<OnboardingStatus> for i32 {
    fn from(status: OnboardingStatus) -> i32 {
        status as i32
    }
}

impl OnboardingStatus {
    pub fn is_initial_sync_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub firstname: String,
    pub avatar: Option<String>,
    pub is_admin: bool,
    pub onboarding_status: OnboardingStatus,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct UserPublic {
    pub id: UserId,
    pub name: String,
    pub firstname: String,
    pub avatar: Option<String>,
}

impl IdKeyed for UserPublic {
    type Key = UserId;
    fn key(&self) -> Self::Key {
        self.id
    }
}

/// The user with its state: the full [`User`] plus the [`Summary`] of
/// everything the user owns — parts, usages, attachments, services, plans,
/// part notes, shops, activities, and the shop users — optionally filtered
/// by the `?shop=` parameter. The one primitive a read driver asks for
/// (spec #558 §3.5): the per-user routes project `User`, `Summary`, and the
/// export composite from it, and `UserId::read` is the single op that
/// returns it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UserState {
    /// The full user record (admin flag and onboarding status included —
    /// the `Summary`'s `UserPublic` entries carry neither).
    pub user: User,
    /// The user's state, as `UserId::get_summary` computes it.
    pub summary: Summary,
}

impl From<User> for UserPublic {
    fn from(value: User) -> Self {
        let User {
            id,
            name,
            firstname,
            avatar,
            ..
        } = value;
        Self {
            id,
            name,
            firstname,
            avatar,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Stat {
    pub user: User,
    parts: i64,
    activities: i64,
}

impl UserId {
    /// The unified user read (spec #558 §3.5): the user with its state —
    /// the full [`User`] plus the [`Summary`] of everything the user owns,
    /// optionally filtered by `shop` (the route's `?shop=` parameter).
    /// Bare `User` and `Summary` are projections of the [`UserState`].
    ///
    /// Crosses: activity, attachment, part, partnote, service, serviceplan,
    /// shop, usage, user.
    pub async fn read(self, shop: Option<ShopId>, store: &mut impl Store) -> TbResult<UserState> {
        let user = UserStore::get(store, self).await?;
        let summary = self.get_summary(shop, store).await?;
        Ok(UserState { user, summary })
    }

    pub async fn get_public(self, store: &mut impl UserStore) -> TbResult<UserPublic> {
        let User {
            id,
            name,
            firstname,
            avatar,
            ..
        } = store.get(self).await?;
        Ok(UserPublic {
            id,
            name,
            firstname,
            avatar,
        })
    }

    pub async fn get_stat(
        &self,
        store: &mut (impl ActivityStore + PartStore + UserStore),
    ) -> TbResult<Stat> {
        // The lightweight single-row read: the stat needs the user record,
        // not the `UserState` composite (the composite is the unified
        // read's primitive, spec #558 §3.5 — the per-user routes only).
        let user = UserStore::get(store, *self).await.context("User record")?;
        let parts = Part::get_all(self, store)
            .await
            .context("User parts")?
            .len() as i64;
        let activities = Activity::get_all(self, store)
            .await
            .context("User activities")?
            .len() as i64;
        Ok(Stat {
            user,
            parts,
            activities,
        })
    }

    pub async fn create(
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        store: &mut impl UserStore,
    ) -> TbResult<Self> {
        store
            .create(firstname, lastname, avatar)
            .await
            .map(|u| u.id)
    }

    pub async fn update(
        &self,
        firstname_: &str,
        lastname: &str,
        avatar: &Option<String>,
        store: &mut impl UserStore,
    ) -> TbResult<Self> {
        store
            .update(self, firstname_, lastname, avatar)
            .await
            .map(|u| u.id)
    }

    pub async fn is_admin(&self, store: &mut impl UserStore) -> TbResult<bool> {
        // The lightweight single-row read: one bool, no `UserState`
        // composite (this runs at every login and every executor spawn).
        UserStore::get(store, *self).await.map(|u| u.is_admin)
    }

    /// get all parts, attachments and activities for the user
    ///
    /// Crosses: activity, attachment, part, partnote, service, serviceplan, shop, usage, user.
    pub async fn get_summary(
        &self,
        shop: Option<ShopId>,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        use crate::*;
        let activities = Activity::get_all(self, store).await?;
        let shops = Shop::get_all_for_user(self, store).await?;
        let users = Shop::get_users(&shops, self, store).await?;
        let mut summary = {
            let parts = match shop {
                None => Part::get_all(self, store).await?,
                Some(shop) => shop.get_parts(*self, store).await?,
            };
            self.get_part_summary(parts, store).await?
        };
        summary.activities.upsert_all(activities);
        summary.shops.upsert_all(shops);
        summary.users.upsert_all(users);
        Ok(summary)
    }

    /// Returns a summary for the user self and the list of parts provided
    ///
    /// # Arguments
    ///
    /// * `parts` - list of parts
    /// * `store` - A mutable reference to an `AppConn` object representing the database connection.
    ///
    /// # Returns
    ///
    /// A `Summary` with all entities related to parts
    ///
    /// # Errors
    ///
    /// Returns an `TbResult` object that may contain a `diesel::result::Error` if the query fails.
    async fn get_part_summary(
        &self,
        parts: Vec<Part>,
        store: &mut (
                 impl AttachmentStore
                 + PartNoteStore
                 + PartStore
                 + ServicePlanStore
                 + ServiceStore
                 + UsageStore
             ),
    ) -> TbResult<Summary> {
        let mut usages = Vec::new();
        let mut attachments = Vec::new();
        let mut services = Vec::new();
        let mut part_notes = Vec::new();
        let mut plans = ServicePlan::for_user(self, store).await?;
        for part in &parts {
            usages.push(part.usage().read(store).await?);
            let (mut atts, mut uses) = Attachment::for_part_with_usage(part.id, store).await?;
            usages.append(&mut uses);
            attachments.append(&mut atts);
            let (mut servs, mut uses) = Service::for_part_with_usage(part.id, store).await?;
            let mut splans = ServicePlan::for_part(part.id, store).await?;
            usages.append(&mut uses);
            services.append(&mut servs);
            plans.append(&mut splans);
            part_notes.append(&mut store.partnote_all_by_part(part.id).await?);
        }
        let mut summary = Summary::default();
        summary.parts.upsert_all(parts);
        summary.usages.upsert_all(usages);
        summary.attachments.upsert_all(attachments);
        summary.services.upsert_all(services);
        summary.part_notes.upsert_all(part_notes);
        summary.plans.upsert_all(plans);
        Ok(summary)
    }

    /// Deletes the user plus their activities, parts, attachments, services, serviceplans and usages
    ///
    /// Crosses: activity, attachment, part, partnote, service, serviceplan, shop, usage, user.
    pub async fn delete(&self, store: &mut impl Store) -> TbResult<()> {
        let summary = self.get_summary(None, store).await?;
        let services = summary.services.live();
        let plans = summary.plans.live();
        let parts = summary.parts.live();
        let activities = summary.activities.live();
        let usages = summary.usages.live();
        let n = store.services_delete(&services).await?;
        debug!("deleted {n} services");
        let n = store.serviceplans_delete(&plans).await?;
        debug!("deleted {n} serviceplans");
        let n = store.attachments_delete_by_parts(&parts).await?;
        debug!("deleted {n} attachments");
        let n = store.activities_delete(&activities).await?;
        debug!("deleted {n} activities");
        let n = store.parts_delete(&parts).await?;
        debug!("deleted {n} parts");
        let n = store.usages_delete(&usages).await?;
        debug!("deleted {n} usages");
        let n = store.user_delete(self).await?;
        debug!("deleted {n} user");
        Ok(())
    }

    pub(crate) fn check_owner(&self, owner: UserId, error: String) -> TbResult<()> {
        if *self == owner {
            Ok(())
        } else {
            Err(crate::Error::Forbidden(error))
        }
    }
}

#[cfg(test)]
mod user_tests;
