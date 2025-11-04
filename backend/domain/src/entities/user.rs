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
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};

use crate::*;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(i32);

NewtypeDisplay! { () pub struct UserId(); }
NewtypeFrom! { () pub struct UserId(i32); }

/// Onboarding status enum for tracking user setup progress
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(i32)]
pub enum OnboardingStatus {
    /// User has not completed initial activity sync
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

impl Default for OnboardingStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub firstname: String,
    pub avatar: Option<String>,
    pub is_admin: bool,
    pub onboarding_status: OnboardingStatus,
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
        let user = self.read(store).await.context("User record")?;
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

    pub async fn delete(&self, store: &mut impl Store) -> TbResult<()> {
        let Summary {
            activities,
            parts,
            usages,
            services,
            plans,
            ..
        } = self.get_summary(store).await?;
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
}

impl Person for User {
    fn get_id(&self) -> UserId {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}
