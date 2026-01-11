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

use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

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
            created_at: shop.created_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopId(i32);

NewtypeDisplay! { () pub struct ShopId(); }
NewtypeFrom! { () pub struct ShopId(i32); }

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
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        store.shop_create(name, description, user).await
    }

    /// Update an existing shop
    pub async fn update(
        self,
        name: String,
        description: Option<String>,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        self.checkowner(user, store).await?;
        store.shop_update(self, name, description).await
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
    ) -> TbResult<crate::Summary> {
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
    /// Allows access for owners, and active subscribers
    pub async fn read(self, user: UserId, store: &mut impl Store) -> TbResult<Shop> {
        self.check_read_access(user, store).await?;
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
    pub async fn with_owner_info(
        shops: Vec<Shop>,
        store: &mut impl Store,
    ) -> TbResult<Vec<ShopWithOwner>> {
        let mut result = Vec::new();
        for shop in shops {
            let owner = shop.owner.read(store).await?;
            result.push(ShopWithOwner::from_shop_and_user(shop, owner));
        }
        Ok(result)
    }
}

/// Subscription status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Pending,
    Active,
    Rejected,
    Cancelled,
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionStatus::Pending => write!(f, "pending"),
            SubscriptionStatus::Active => write!(f, "active"),
            SubscriptionStatus::Rejected => write!(f, "rejected"),
            SubscriptionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A subscription to a shop, allowing a user to register their bikes
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShopSubscription {
    pub id: SubscriptionId,
    pub shop_id: ShopId,
    pub user_id: UserId,
    pub status: SubscriptionStatus,
    pub message: Option<String>,
    pub response_message: Option<String>,
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde_as(as = "Rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// A subscription with shop details for API responses
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShopSubscriptionWithDetails {
    pub id: SubscriptionId,
    pub shop_id: ShopId,
    pub shop_name: String,
    pub shop_owner_firstname: String,
    pub shop_owner_name: String,
    pub user_id: UserId,
    pub status: SubscriptionStatus,
    pub message: Option<String>,
    pub response_message: Option<String>,
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde_as(as = "Rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl ShopSubscriptionWithDetails {
    /// Create from subscription and shop with owner
    pub fn from_subscription_and_shop(subscription: ShopSubscription, shop: ShopWithOwner) -> Self {
        Self {
            id: subscription.id,
            shop_id: subscription.shop_id,
            shop_name: shop.name,
            shop_owner_firstname: shop.owner_firstname,
            shop_owner_name: shop.owner_name,
            user_id: subscription.user_id,
            status: subscription.status,
            message: subscription.message,
            response_message: subscription.response_message,
            created_at: subscription.created_at,
            updated_at: subscription.updated_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionId(i32);

NewtypeDisplay! { () pub struct SubscriptionId(); }
NewtypeFrom! { () pub struct SubscriptionId(i32); }

impl SubscriptionId {
    /// Create a new subscription request
    pub async fn create(
        shop_id: ShopId,
        message: Option<String>,
        user: UserId,
        store: &mut impl Store,
    ) -> TbResult<ShopSubscription> {
        // Verify the shop exists (don't check ownership - users can subscribe to any shop)
        store.shop_get(shop_id).await?;

        // Check if there's already a pending subscription
        let existing = store.subscription_find_pending(shop_id, user).await?;
        if existing.is_some() {
            return Err(Error::Conflict(
                "A pending subscription request already exists".into(),
            ));
        }

        // Check if there's already an active subscription
        let active = store.subscription_find_active(shop_id, user).await?;
        if active.is_some() {
            return Err(Error::Conflict(
                "You are already subscribed to this shop".into(),
            ));
        }

        store.subscription_create(shop_id, user, message).await
    }

    /// Get a subscription by ID
    pub async fn get(id: i32, user: UserId, store: &mut impl Store) -> TbResult<SubscriptionId> {
        SubscriptionId(id).checkuser(user, store).await
    }

    /// Read a subscription from the database
    pub async fn read(self, user: UserId, store: &mut impl Store) -> TbResult<ShopSubscription> {
        self.checkuser(user, store).await?;
        store.subscription_get(self).await
    }

    /// Check if the user has access to this subscription (either subscriber or shop owner)
    pub async fn checkuser(self, user: UserId, store: &mut impl Store) -> TbResult<SubscriptionId> {
        let subscription = store.subscription_get(self).await?;

        // Allow access if user is the subscriber
        if subscription.user_id == user {
            return Ok(self);
        }

        // Allow access if user owns the shop
        let shop = store.shop_get(subscription.shop_id).await?;
        user.check_owner(shop.owner, "Access denied to subscription".to_string())?;

        Ok(self)
    }

    /// Approve a subscription (shop owner only)
    pub async fn approve(
        self,
        response_message: Option<String>,
        user: UserId,
        store: &mut impl Store,
    ) -> TbResult<ShopSubscription> {
        let subscription = store.subscription_get(self).await?;

        // Verify user owns the shop
        let shop_id = subscription.shop_id;
        shop_id.checkowner(user, store).await?;

        if subscription.status != SubscriptionStatus::Pending {
            return Err(Error::Conflict("Subscription is not pending".into()));
        }

        // Update subscription status to active with response message
        store
            .subscription_approve(self, SubscriptionStatus::Active, response_message)
            .await
    }

    /// Reject a subscription (shop owner only)
    pub async fn reject(
        self,
        response_message: Option<String>,
        user: UserId,
        store: &mut impl Store,
    ) -> TbResult<ShopSubscription> {
        let subscription = store.subscription_get(self).await?;

        // Verify user owns the shop
        let shop_id = subscription.shop_id;
        shop_id.checkowner(user, store).await?;

        if subscription.status != SubscriptionStatus::Pending {
            return Err(Error::Conflict("Subscription is not pending".into()));
        }

        store
            .subscription_approve(self, SubscriptionStatus::Rejected, response_message)
            .await
    }

    /// Cancel a subscription (subscriber only)
    /// Allows deletion of pending, active, and rejected subscriptions
    pub async fn cancel(self, user: UserId, store: &mut impl Store) -> TbResult<()> {
        let subscription = store.subscription_get(self).await?;

        // Verify user is the subscriber
        user.check_owner(
            subscription.user_id,
            "Access denied - not the subscriber".to_string(),
        )?;

        if subscription.status != SubscriptionStatus::Pending
            && subscription.status != SubscriptionStatus::Active
            && subscription.status != SubscriptionStatus::Rejected
        {
            return Err(Error::Conflict(
                "Can only cancel pending, active, or rejected subscriptions".into(),
            ));
        }

        if store
            .shop_get_parts(subscription.shop_id)
            .await?
            .iter()
            .any(|p| p.owner == user)
        {
            return Err(Error::Conflict("You have still parts in the shop".into()));
        }

        store.subscription_delete(self).await
    }
}

impl ShopSubscription {
    /// Get all pending subscriptions for a shop (shop owner only)
    pub async fn get_pending_for_shop(
        shop_id: ShopId,
        user: UserId,
        store: &mut impl Store,
    ) -> TbResult<Vec<ShopSubscription>> {
        shop_id.checkowner(user, store).await?;
        store
            .subscriptions_for_shop(shop_id, Some(SubscriptionStatus::Pending))
            .await
    }

    /// Get all subscriptions made by a user
    pub async fn get_for_user(
        user: UserId,
        store: &mut impl Store,
    ) -> TbResult<Vec<ShopSubscription>> {
        store.subscriptions_for_user(user).await
    }

    /// Convert a list of subscriptions to subscriptions with shop details
    pub async fn with_shop_details(
        subscriptions: Vec<ShopSubscription>,
        store: &mut impl Store,
    ) -> TbResult<Vec<ShopSubscriptionWithDetails>> {
        let mut result = Vec::new();
        for subscription in subscriptions {
            let shop = store.shop_get(subscription.shop_id).await?;
            let owner = shop.owner.read(store).await?;
            let shop_with_owner = ShopWithOwner::from_shop_and_user(shop, owner);
            result.push(ShopSubscriptionWithDetails::from_subscription_and_shop(
                subscription,
                shop_with_owner,
            ));
        }
        Ok(result)
    }
}
