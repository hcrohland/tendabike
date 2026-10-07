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
use std::collections::HashMap;

use crate::*;

/// Flatten an id-keyed `Summary` map to a `Vec` of live entities, dropping tombstones.
fn live_values<K, V>(m: HashMap<K, Option<V>>) -> Vec<V> {
    m.into_values().flatten().collect()
}

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
    pub async fn read(self, store: &mut impl UserStore) -> TbResult<User> {
        store.get(self).await
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
        let summary = {
            let parts = match shop {
                None => Part::get_all(self, store).await?,
                Some(shop) => shop.get_parts(*self, store).await?,
            };
            self.get_part_summary(parts, store).await?
        };
        Ok(Summary {
            activities: activities.into_iter().map(|a| (a.id, Some(a))).collect(),
            shops: shops.into_iter().map(|s| (s.id, Some(s))).collect(),
            users: users.into_iter().map(|u| (u.id, Some(u))).collect(),
            ..summary
        })
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
        Ok(Summary {
            parts: parts.into_iter().map(|p| (p.id, Some(p))).collect(),
            usages: usages.into_iter().map(|u| (u.id, Some(u))).collect(),
            attachments: attachments
                .into_iter()
                .map(|a| (a.idx(), Some(a)))
                .collect(),
            services: services.into_iter().map(|s| (s.id, Some(s))).collect(),
            part_notes: part_notes.into_iter().map(|n| (n.id, Some(n))).collect(),
            plans: plans.into_iter().map(|p| (p.id, Some(p))).collect(),
            ..Default::default()
        })
    }

    /// Deletes the user plus their activities, parts, attachments, services, serviceplans and usages
    ///
    /// Crosses: activity, attachment, part, partnote, service, serviceplan, shop, usage, user.
    pub async fn delete(&self, store: &mut impl Store) -> TbResult<()> {
        let summary = self.get_summary(None, store).await?;
        let services = live_values(summary.services);
        let plans = live_values(summary.plans);
        let parts = live_values(summary.parts);
        let activities = live_values(summary.activities);
        let usages = live_values(summary.usages);
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
mod tests {
    use super::*;

    #[test]
    fn onboarding_status_try_from_valid() {
        assert_eq!(
            OnboardingStatus::try_from(0i32).unwrap(),
            OnboardingStatus::Pending
        );
        assert_eq!(
            OnboardingStatus::try_from(2i32).unwrap(),
            OnboardingStatus::InitialSyncPostponed
        );
        assert_eq!(
            OnboardingStatus::try_from(99i32).unwrap(),
            OnboardingStatus::Completed
        );
    }

    #[test]
    fn onboarding_status_try_from_invalid() {
        assert!(matches!(
            OnboardingStatus::try_from(1i32).unwrap_err(),
            Error::BadRequest(_)
        ));
        assert!(matches!(
            OnboardingStatus::try_from(5i32).unwrap_err(),
            Error::BadRequest(_)
        ));
        assert!(matches!(
            OnboardingStatus::try_from(-1i32).unwrap_err(),
            Error::BadRequest(_)
        ));
    }

    #[test]
    fn onboarding_status_from_i32_roundtrip() {
        for status in [
            OnboardingStatus::Pending,
            OnboardingStatus::InitialSyncPostponed,
            OnboardingStatus::Completed,
        ] {
            let val = i32::from(status);
            assert_eq!(OnboardingStatus::try_from(val).unwrap(), status);
        }
    }

    #[test]
    fn onboarding_status_is_initial_sync_completed() {
        assert!(!OnboardingStatus::Pending.is_initial_sync_completed());
        assert!(!OnboardingStatus::InitialSyncPostponed.is_initial_sync_completed());
        assert!(OnboardingStatus::Completed.is_initial_sync_completed());
    }

    #[test]
    fn user_public_drops_admin_and_onboarding() {
        let user = User {
            id: UserId::from(1i32),
            name: "Doe".into(),
            firstname: "John".into(),
            avatar: Some("http://example.com/a.png".into()),
            is_admin: true,
            onboarding_status: OnboardingStatus::Completed,
        };
        let pub_ = UserPublic::from(user);
        let json = serde_json::to_string(&pub_).unwrap();
        assert!(!json.contains("is_admin"));
        assert!(!json.contains("onboarding_status"));
        assert!(json.contains("\"name\":\"Doe\""));
        assert!(json.contains("\"firstname\":\"John\""));
    }

    #[test]
    fn user_id_check_owner_same() {
        let id = UserId::from(42i32);
        assert!(id.check_owner(UserId::from(42i32), "nope".into()).is_ok());
    }

    #[test]
    fn user_id_check_owner_different() {
        let id = UserId::from(42i32);
        let err = id
            .check_owner(UserId::from(43i32), "not your resource".into())
            .unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    // === Tier B: User CRUD (in-memory store) ===

    use crate::test_support::MemStore;

    #[tokio::test]
    async fn user_create_and_read() {
        let mut store = MemStore::new();
        let uid = UserId::create("John", "Doe", &None, &mut store)
            .await
            .unwrap();
        let user = uid.read(&mut store).await.unwrap();
        assert_eq!(user.id, uid);
        assert_eq!(user.firstname, "John");
        assert_eq!(user.name, "Doe");
        assert!(!user.is_admin);
        assert_eq!(user.onboarding_status, OnboardingStatus::Pending);
    }

    #[tokio::test]
    async fn user_get_public() {
        let mut store = MemStore::new();
        let uid = UserId::create("Jane", "Smith", &Some("avatar.png".into()), &mut store)
            .await
            .unwrap();
        let pub_ = uid.get_public(&mut store).await.unwrap();
        assert_eq!(pub_.id, uid);
        assert_eq!(pub_.firstname, "Jane");
        assert_eq!(pub_.name, "Smith");
        assert_eq!(pub_.avatar, Some("avatar.png".into()));
    }

    #[tokio::test]
    async fn user_update() {
        let mut store = MemStore::new();
        let uid = UserId::create("Alice", "Bob", &None, &mut store)
            .await
            .unwrap();
        uid.update("Alice", "NewName", &Some("new.png".into()), &mut store)
            .await
            .unwrap();
        let user = uid.read(&mut store).await.unwrap();
        assert_eq!(user.firstname, "Alice");
        assert_eq!(user.name, "NewName");
        assert_eq!(user.avatar, Some("new.png".into()));
    }

    #[tokio::test]
    async fn user_is_admin_false_by_default() {
        let mut store = MemStore::new();
        let uid = UserId::create("Bob", "Builder", &None, &mut store)
            .await
            .unwrap();
        assert!(!uid.is_admin(&mut store).await.unwrap());
    }

    #[tokio::test]
    async fn user_delete_removes_from_store() {
        let mut store = MemStore::new();
        let uid = UserId::create("Del", "User", &None, &mut store)
            .await
            .unwrap();
        uid.delete(&mut store).await.unwrap();
        assert!(uid.read(&mut store).await.is_err());
    }

    #[tokio::test]
    async fn user_update_onboarding_status() {
        let mut store = MemStore::new();
        let uid = UserId::create("Carol", "C", &None, &mut store)
            .await
            .unwrap();
        store
            .update_onboarding_status(&uid, OnboardingStatus::Completed)
            .await
            .unwrap();
        let user = uid.read(&mut store).await.unwrap();
        assert_eq!(user.onboarding_status, OnboardingStatus::Completed);
    }

    // === Tier C: User summary (prepopulated fixture) ===

    use crate::test_support::part_type_ids::BIKE;

    /// The user summary read returns the full prepopulated fixture content
    /// — the in-memory twin of the store-seam suite's `user_summary_read`
    /// (which asserts the same thing over the same fixture loaded into
    /// real Postgres).
    #[tokio::test]
    async fn user_summary_read() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let summary = UserId::from(1).get_summary(None, &mut store).await?;

        // The fixture content. Parts come back in one unified order on
        // both stores — ascending `last_used` (#405) — but the summary map
        // is unordered, so the assertion below sorts the part values by
        // `last_used` and checks the rule on the sorted sequence. See the
        // seam's `user_summary_read`.
        assert_eq!(summary.parts.len(), 17);
        let mut parts: Vec<_> = summary.parts.values().flatten().collect();
        parts.sort_by_key(|p| p.last_used);
        for (earlier, later) in parts.iter().zip(parts.iter().skip(1)) {
            assert!(
                earlier.last_used <= later.last_used,
                "parts not sorted by last_used"
            );
        }
        assert_eq!(summary.activities.len(), 3);
        assert_eq!(summary.attachments.len(), 11);
        // 17 part usages + 11 attachment usages (missing usage rows read
        // as zeros, identically on both stores).
        assert_eq!(summary.usages.len(), 28);
        assert!(summary.shops.is_empty());
        assert!(summary.users.is_empty());
        assert!(summary.services.is_empty());
        assert!(summary.plans.is_empty());
        assert!(summary.part_notes.is_empty());

        // Field lookups: the "Chain A" part, its attachment, and its usage.
        let chain = summary
            .parts
            .values()
            .flatten()
            .find(|p| p.id == PartId::from(4))
            .unwrap();
        assert_eq!(chain.name, "Chain A");
        let att = summary
            .attachments
            .values()
            .flatten()
            .find(|a| a.a.part_id == PartId::from(4))
            .unwrap();
        assert_eq!(att.a.hook, BIKE);
        assert_eq!(att.a.detached, MAX_TIME);
        let usage = summary
            .usages
            .values()
            .flatten()
            .find(|u| u.id == chain.usage)
            .unwrap();
        assert_eq!(usage.time, 8025);
        assert_eq!(usage.count, 3);

        Ok(())
    }
}
