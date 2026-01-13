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

//! This module contains the tb_domain logic for shops in the Tendabike system.
//!
//! A `Shop` represents a collection of bikes owned by a shop owner. Users can register
//! their bikes to a shop, delegating maintenance to the shop owner.

use std::collections::HashMap;

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

pub mod subscription;
pub use subscription::*;

/// The database's representation of a shop.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shop {
    /// The primary key
    pub id: ShopId,
    /// The owner of the shop
    pub owner: UserId,
    /// The name of the shop
    pub name: String,
    /// Optional description of the shop
    pub description: Option<String>,
    /// If registration requests need approval
    pub auto_approve: bool,
    /// Creation timestamp
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Shop with owner information for API responses
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShopWithOwner {
    /// The primary key
    pub id: ShopId,
    /// The owner ID of the shop
    pub owner: UserId,
    /// The owner's first name
    pub owner_firstname: String,
    /// The owner's last name
    pub owner_name: String,
    /// The name of the shop
    pub name: String,
    /// Optional description of the shop
    pub description: Option<String>,
    /// If registration requests need approval
    pub auto_approve: bool,
    /// Creation timestamp
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

impl ShopWithOwner {
    /// Create a ShopWithOwner from a Shop and User
    pub fn from_shop_and_user(shop: Shop, user: User) -> Self {
        Self {
            id: shop.id,
            owner: shop.owner,
            owner_firstname: user.firstname,
            owner_name: user.name,
            name: shop.name,
            description: shop.description,
            auto_approve: shop.auto_approve,
            created_at: shop.created_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopId(i32);

impl ShopId {
    /// Get a shop by ID, checking that the user has access to it (ownership only)
    pub async fn get(id: i32, user: UserId, store: &mut impl ShopStore) -> TbResult<ShopId> {
        ShopId(id).checkowner(user, store).await
    }

    /// Get a shop by ID for read access (owner, or active subscriber)
    pub async fn get_for_read(id: i32, user: UserId, store: &mut impl Store) -> TbResult<ShopId> {
        ShopId(id).check_read_access(user, store).await
    }

    /// Check if the user owns this shop
    pub async fn checkowner(self, user: UserId, store: &mut impl ShopStore) -> TbResult<ShopId> {
        let shop = store.shop_get(self).await?;
        user.check_owner(shop.owner, "Access denied to shop".to_string())?;
        Ok(self)
    }

    /// Check if the user has read access to this shop (owner, or active subscriber)
    pub async fn check_read_access(self, user: UserId, store: &mut impl Store) -> TbResult<ShopId> {
        let shop = store.shop_get(self).await?;
        let is_owner = shop.owner == user;

        if is_owner || self.has_subscription(user, store).await? {
            return Ok(self);
        }

        Err(Error::Forbidden("Access denied to shop".into()))
    }

    /// Create a new shop
    pub async fn create(
        name: String,
        description: Option<String>,
        auto_approve: bool,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        store
            .shop_create(name, description, auto_approve, user)
            .await
    }

    /// Update an existing shop
    pub async fn update(
        self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        self.checkowner(user, store).await?;
        store
            .shop_update(self, name, description, auto_approve)
            .await
    }

    /// Delete a shop (only if it has no bikes)
    pub async fn delete(self, user: UserId, store: &mut impl Store) -> TbResult<ShopId> {
        self.checkowner(user, store).await?;

        // Check if shop has any bikes
        let parts = store.shop_get_parts(self).await?;
        if !parts.is_empty() {
            return Err(Error::Conflict("Shop still has bikes assigned".into()));
        }

        store.shop_delete(self).await?;
        Ok(self)
    }

    /// Register a part (bike) to this shop
    /// Can be done by shop owner or any user with an active subscription
    /// Automatically registers all currently attached parts (cascading registration)
    /// Returns a Summary with the registered part and its attachments
    pub async fn register_part(
        self,
        part_id: PartId,
        session: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let parts = parts_for_register(part_id, session, store).await?;

        // Register the parts to the shop
        let parts = store.parts_register_shop(self, parts).await?;

        Ok(Summary {
            parts,
            ..Default::default()
        })
    }

    async fn has_subscription(self, user: UserId, store: &mut impl Store) -> Result<bool, Error> {
        Ok(store.subscription_find_active(self, user).await?.is_some())
    }

    /// Unregister a part (bike) from this shop
    /// Can be done by shop owner OR part owner
    /// Returns an empty Summary (for consistency with other endpoints)
    pub async fn unregister_part(
        self,
        part_id: PartId,
        session: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let parts = parts_for_register(part_id, session, store).await?;

        let parts = store.parts_unregister_shop(parts).await?;

        Ok(Summary {
            parts,
            ..Default::default()
        })
    }

    /// Get all parts and their subparts registered to this shop
    /// Can be accessed by shop owner
    pub async fn get_parts(self, user: UserId, store: &mut impl Store) -> TbResult<Vec<Part>> {
        // Check if user is shop owner OR has active subscription
        self.check_owner(user, store).await?;

        store.shop_get_parts(self).await
    }

    /// Read a shop from the database
    /// Everybiódy should be able to read this
    pub async fn read(self, store: &mut impl Store) -> TbResult<Shop> {
        store.shop_get(self).await
    }

    pub(crate) async fn check_owner(&self, user: UserId, store: &mut impl Store) -> TbResult<()> {
        let shop = store.shop_get(*self).await?;
        if user != shop.owner {
            return Err(Error::Forbidden(
                "You must be the shop owner to access this shop".into(),
            ));
        }
        Ok(())
    }
}

async fn parts_for_register(
    part_id: PartId,
    session: &dyn Session,
    store: &mut impl Store,
) -> TbResult<Vec<PartId>> {
    part_id.checkuser(session, store).await?;
    let time = OffsetDateTime::now_utc();
    if is_attached(part_id, time, store).await? {
        return Err(Error::BadRequest(
            "You can only register parts which are not attached".to_string(),
        ));
    };
    let mut parts = subparts(part_id, time, store).await?;
    parts.push(part_id);
    Ok(parts)
}

impl Shop {
    /// Get all shops for a user
    pub async fn get_all_for_user(
        user: &UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Vec<Shop>> {
        store.shops_get_all_for_user(*user).await
    }

    /// Search for shops by name (for users to find shops to request registration)
    pub async fn search(query: &str, store: &mut impl ShopStore) -> TbResult<Vec<Shop>> {
        store.shops_search(query).await
    }

    /// Convert a list of shops to shops with owner information
    pub async fn get_users(
        shops: &Vec<Shop>,
        user: &UserId,
        store: &mut impl Store,
    ) -> TbResult<Vec<UserPublic>> {
        let mut result: HashMap<UserId, UserPublic> = HashMap::new();
        for shop in shops {
            let owner = shop.owner.get_public(store).await?;
            result.insert(owner.id, owner);
            if *user == shop.owner {
                shop.add_subscribers(&mut result, store).await?;
            }
        }
        Ok(result.into_values().collect())
    }

    async fn add_subscribers(
        &self,
        result: &mut HashMap<UserId, UserPublic>,
        store: &mut impl Store,
    ) -> TbResult<()> {
        for subscription in ShopSubscription::get_for_shop(self.id, store).await? {
            let user = subscription.user_id.get_public(store).await?;
            result.insert(user.id, user);
        }
        Ok(())
    }
}
