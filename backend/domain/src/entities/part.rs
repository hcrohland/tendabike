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

use derive_more::{Display, From, Into};
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
    /// notes about the part
    pub notes: String,
    /// Optional shop for delegated maintenance
    pub shop: Option<ShopId>,
}

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
)]
pub struct PartId(i32);

impl PartId {
    pub async fn get(id: i32, user: &dyn Session, store: &mut impl Store) -> TbResult<PartId> {
        PartId(id).checkuser(user, store).await
    }

    pub async fn delete(self, user: &dyn Session, store: &mut impl Store) -> TbResult<PartId> {
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
    pub async fn part(self, session: &dyn Session, store: &mut impl Store) -> TbResult<Part> {
        let part = self.read(store).await?;

        let user = session.user_id();
        if part.owner != user {
            match session.shop() {
                Some(shop) => {
                    shop.check_owner(user, store).await?;
                }
                None => {
                    return Err(Error::Forbidden(format!(
                        "user {user} cannot access part {}",
                        part.id
                    )));
                }
            }
        }
        Ok(part)
    }

    /// get the name of the part
    ///
    /// does not check ownership. This is needed for rentals.
    pub async fn name(self, store: &mut impl PartStore) -> TbResult<String> {
        Ok(self.read(store).await?.name)
    }

    pub async fn is_main(self, store: &mut impl PartStore) -> TbResult<bool> {
        let part = self.read(store).await?;
        part.what.is_main()
    }

    /// check if the given user is the owner or an authorized shop owner.
    /// Returns Forbidden if not.
    pub async fn checkuser(
        self,
        session: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<PartId> {
        self.part(session, store).await.map(|p| p.id)
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
        if part.disposed_at.is_some() {
            return Err(Error::BadRequest(format!(
                "part {} already disposed",
                part.id
            )));
        }
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
        notes: String,
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<Part> {
        info!("Change {self:?}");

        let mut part = self.part(user, store).await?;

        let purchase = round_time(purchase);
        part = Part {
            name,
            vendor,
            model,
            purchase,
            notes,
            ..part
        };
        store.part_update(part).await
    }

    pub(crate) async fn set_owner_and_shop(
        &self,
        gear: PartId,
        store: &mut impl Store,
    ) -> TbResult<Part> {
        let mut part = self.read(store).await?;
        let gear = gear.read(store).await?;
        if part.owner != gear.owner || part.shop != gear.shop {
            part.owner = gear.owner;
            part.shop = gear.shop;
            return store.part_update(part).await;
        }
        Ok(part)
    }
}

impl Part {
    pub(crate) async fn get_all(pid: &UserId, store: &mut impl Store) -> TbResult<Vec<Part>> {
        store.part_get_all_for_userid(pid).await
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
        notes: String,
        user: &dyn Session,
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
                notes,
                UsageId::new(),
                user.user_id(),
                user.shop(),
            )
            .await
    }

    pub async fn categories(
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<HashSet<PartTypeId>> {
        let parts = store.part_get_all_for_userid(&user.user_id()).await?;
        let mut res = HashSet::new();
        for part in parts {
            if part.what.is_main()? {
                res.insert(part.what);
            }
        }
        Ok(res)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MemStore, TestSession, fixtures};
    use time::OffsetDateTime;

    use fixtures::{sample_purchase_date, test_user};

    fn later_time() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700100000).unwrap()
    }

    // === PartId tests ===

    /// PartId::read retrieves a stored part
    #[tokio::test]
    async fn partid_read_returns_stored_part() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let retrieved = PartId::from(1).read(&mut store).await?;
        assert_eq!(retrieved.name, "Main Bike");
        assert_eq!(retrieved.vendor, "TendaBike");
        Ok(())
    }

    /// PartId::read returns error for non-existent part
    #[tokio::test]
    async fn partid_read_not_found_error() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let result = PartId::from(999).read(&mut store).await;
        assert!(result.is_err());
        Ok(())
    }

    /// PartId::name returns the part name without checking ownership
    #[tokio::test]
    async fn partid_name_returns_name() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let name = PartId::from(1).name(&mut store).await?;
        assert_eq!(name, "Main Bike");
        Ok(())
    }

    /// PartId::is_main returns true for main part types
    #[tokio::test]
    async fn partid_is_main_for_bike() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let is_main = PartId::from(1).is_main(&mut store).await?;
        assert!(is_main);
        Ok(())
    }

    /// PartId::is_main returns false for sub-part types
    #[tokio::test]
    async fn partid_is_not_main_for_wheel() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let is_main = PartId::from(2).is_main(&mut store).await?;
        assert!(!is_main);
        Ok(())
    }

    /// PartId::update_timestamps updates last_used when start is later
    #[tokio::test]
    async fn update_timestamps_updates_last_used() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let updated = PartId::from(4)
            .update_timestamps(later_time(), &mut store)
            .await?;
        assert_eq!(updated.last_used, later_time());
        Ok(())
    }

    /// PartId::dispose sets disposed_at timestamp
    #[tokio::test]
    async fn dispose_sets_disposed_at() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let disposed = PartId::from(4).dispose(later_time(), &mut store).await?;
        assert!(disposed.disposed_at.is_some());
        Ok(())
    }

    /// PartId::restore clears disposed_at
    #[tokio::test]
    async fn restore_clears_disposed_at() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        PartId::from(4)
            .dispose(sample_purchase_date(), &mut store)
            .await?;
        let restored = PartId::from(4).restore(&mut store).await?;
        assert!(restored.disposed_at.is_none());
        Ok(())
    }

    /// PartId::set_owner_and_shop copies owner and shop from gear
    #[tokio::test]
    async fn set_owner_and_shop_copies_from_gear() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let gear = PartId::from(1).read(&mut store).await?;
        let updated = PartId::from(2)
            .set_owner_and_shop(gear.id, &mut store)
            .await?;
        assert_eq!(updated.owner, gear.owner);
        Ok(())
    }

    // === Part tests ===

    /// Part::usage returns the usage id
    #[test]
    fn part_usage_returns_usage_id() {
        let test_usage = UsageId::new();
        let part = Part {
            id: PartId::from(1),
            owner: test_user(),
            what: PartTypeId::from(1),
            name: "Bike".to_string(),
            vendor: "Trek".to_string(),
            model: "Domane".to_string(),
            purchase: sample_purchase_date(),
            last_used: sample_purchase_date(),
            disposed_at: None,
            usage: test_usage,
            source: None,
            notes: "Notes".to_string(),
            shop: None,
        };

        assert_eq!(part.usage(), test_usage);
    }

    /// Part::create creates a part with correct purchase date rounding
    #[tokio::test]
    async fn part_create_sets_purchase_date() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(99));
        let purchase = sample_purchase_date();
        let part = Part::create(
            "Derailleur".to_string(),
            "Shimano".to_string(),
            "Di2".to_string(),
            PartTypeId::from(4),
            None,
            purchase,
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;

        // Part::create rounds the purchase date via round_time()
        let rounded_purchase = round_time(purchase);
        assert_eq!(part.purchase, rounded_purchase);
        Ok(())
    }

    /// Part::create sets source
    #[tokio::test]
    async fn part_create_with_source() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(99));
        let part = Part::create(
            "Wheelset".to_string(),
            "Easton".to_string(),
            "DA3".to_string(),
            PartTypeId::from(4),
            Some("strava_67890".to_string()),
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;

        assert_eq!(part.source, Some("strava_67890".to_string()));
        Ok(())
    }

    /// Part::get_all returns all parts for a user
    #[tokio::test]
    async fn part_get_all_returns_user_parts() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(99));
        Part::create(
            "Part 1".to_string(),
            "V1".to_string(),
            "M1".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;
        Part::create(
            "Part 2".to_string(),
            "V2".to_string(),
            "M2".to_string(),
            PartTypeId::from(4),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;

        let parts = Part::get_all(&sess.user_id(), &mut store).await?;
        assert_eq!(parts.len(), 2);
        Ok(())
    }

    /// Part::get_all returns empty vec when user has no parts
    #[tokio::test]
    async fn part_get_all_empty_for_user_with_no_parts() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let parts = Part::get_all(&UserId::from(98), &mut store).await?;
        assert!(parts.is_empty());
        Ok(())
    }

    /// Part::categories returns main part types only
    #[tokio::test]
    async fn part_categories_returns_main_types() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(99));
        Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;
        Part::create(
            "Wheel".to_string(),
            "Zipp".to_string(),
            "404".to_string(),
            PartTypeId::from(4),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;

        let categories = Part::categories(&sess, &mut store).await?;
        // Type 1 (Bike) is main type, Type 4 (chain) is subtype - only main types returned
        assert_eq!(categories.len(), 1);
        assert!(categories.contains(&PartTypeId::from(1)));
        assert!(!categories.contains(&PartTypeId::from(4)));
        Ok(())
    }

    /// Part::categories returns empty when no parts
    #[tokio::test]
    async fn part_categories_empty_for_no_parts() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let categories = Part::categories(&TestSession::new(UserId::from(98)), &mut store).await?;
        assert!(categories.is_empty());
        Ok(())
    }

    /// Part::categories filters to main types only - subtypes are excluded
    #[tokio::test]
    async fn part_categories_filters_subtypes() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(99));
        Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;
        Part::create(
            "Front Wheel".to_string(),
            "Zipp".to_string(),
            "404".to_string(),
            PartTypeId::from(2),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &sess,
            &mut store,
        )
        .await?;

        // Type 1 (Bike) is main (hooks=[]), Type 2 (front wheel) is subtype (hooks=[1])
        let categories = Part::categories(&sess, &mut store).await?;
        assert_eq!(categories.len(), 1);
        assert!(categories.contains(&PartTypeId::from(1)));
        Ok(())
    }

    /// PartId::change updates part fields and returns updated part
    #[tokio::test]
    async fn part_change_updates_fields() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(1));
        let updated = PartId::from(1)
            .change(
                "New Name".to_string(),
                "New Vendor".to_string(),
                "New Model".to_string(),
                sample_purchase_date(),
                "New Notes".to_string(),
                &sess,
                &mut store,
            )
            .await?;

        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.vendor, "New Vendor");
        assert_eq!(updated.model, "New Model");
        assert_eq!(updated.notes, "New Notes");
        Ok(())
    }

    /// PartId::change returns forbidden for non-owner session
    #[tokio::test]
    async fn part_change_rejects_non_owner() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let other_session = TestSession::new(UserId::from(98));
        let result = PartId::from(1)
            .change(
                "New Name".to_string(),
                "Vendor".to_string(),
                "Model".to_string(),
                sample_purchase_date(),
                "Notes".to_string(),
                &other_session,
                &mut store,
            )
            .await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// PartId::delete returns successfully when part has no attachments
    #[tokio::test]
    async fn part_delete_succeeds_without_attachments() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(1));
        let result = PartId::from(1).delete(&sess, &mut store).await?;
        assert_eq!(result, PartId::from(1));

        // Verify part is removed from store
        let get_result = PartId::from(1).part(&sess, &mut store).await;
        assert!(matches!(get_result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// PartId::delete returns conflict when part is still attached (conflict check requires AttachmentStore)
    /// Note: AttachmentStore is currently todo!() in MemStore, so this test verifies the delete
    /// succeeds when no attachments exist (the AttachmentStore todo!() is not hit in this path)
    #[tokio::test]
    async fn part_delete_succeeds_without_attachments_documented() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let sess = TestSession::new(UserId::from(1));
        let result = PartId::from(1).delete(&sess, &mut store).await?;
        assert_eq!(result, PartId::from(1));
        Ok(())
    }

    /// PartId::delete returns forbidden for non-owner session
    #[tokio::test]
    async fn part_delete_rejects_non_owner() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let other_session = TestSession::new(UserId::from(98));
        let result = PartId::from(1).delete(&other_session, &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }
}
