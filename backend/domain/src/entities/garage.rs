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

//! This module contains the tb_domain logic for garages in the Tendabike system.
//!
//! A `Garage` represents a collection of bikes owned by a garage owner. Users can register
//! their bikes to a garage, delegating maintenance to the garage owner.

use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

/// The database's representation of a garage.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Garage {
    /// The primary key
    pub id: GarageId,
    /// The owner of the garage
    pub owner: UserId,
    /// The name of the garage
    pub name: String,
    /// Optional description of the garage
    pub description: Option<String>,
    /// Creation timestamp
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Garage summary with related data for garage mode
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarageSummary {
    pub garage: Garage,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentDetail>,
    pub services: Vec<Service>,
    pub plans: Vec<ServicePlan>,
    pub usages: Vec<Usage>,
}

/// Garage with owner information for API responses
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarageWithOwner {
    /// The primary key
    pub id: GarageId,
    /// The owner ID of the garage
    pub owner: UserId,
    /// The owner's first name
    pub owner_firstname: String,
    /// The owner's last name
    pub owner_name: String,
    /// The name of the garage
    pub name: String,
    /// Optional description of the garage
    pub description: Option<String>,
    /// Creation timestamp
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

impl GarageWithOwner {
    /// Create a GarageWithOwner from a Garage and User
    pub fn from_garage_and_user(garage: Garage, user: User) -> Self {
        Self {
            id: garage.id,
            owner: garage.owner,
            owner_firstname: user.firstname,
            owner_name: user.name,
            name: garage.name,
            description: garage.description,
            created_at: garage.created_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GarageId(i32);

NewtypeDisplay! { () pub struct GarageId(); }
NewtypeFrom! { () pub struct GarageId(i32); }

impl GarageId {
    /// Get a garage by ID, checking that the user has access to it (ownership only)
    pub async fn get(
        id: i32,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<GarageId> {
        GarageId(id).checkuser(user, store).await
    }

    /// Get a garage by ID for read access (owner, or active subscriber)
    pub async fn get_for_read(
        id: i32,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageId> {
        GarageId(id).check_read_access(user, store).await
    }

    /// Check if the user owns this garage
    pub async fn checkuser(
        self,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<GarageId> {
        let garage = store.garage_get(self).await?;
        user.check_owner(garage.owner, "Access denied to garage".to_string())?;
        Ok(self)
    }

    /// Check if the user has read access to this garage (owner, or active subscriber)
    pub async fn check_read_access(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageId> {
        let garage = store.garage_get(self).await?;
        let is_owner = garage.owner == user.get_id();

        if is_owner {
            return Ok(self);
        }

        if self.has_subscription(user, store).await? {
            return Ok(self);
        }

        Err(Error::Forbidden("Access denied to garage".into()))
    }

    /// Create a new garage
    pub async fn create(
        name: String,
        description: Option<String>,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<Garage> {
        store.garage_create(name, description, user.get_id()).await
    }

    /// Update an existing garage
    pub async fn update(
        self,
        name: String,
        description: Option<String>,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<Garage> {
        self.checkuser(user, store).await?;
        store.garage_update(self, name, description).await
    }

    /// Delete a garage (only if it has no bikes)
    pub async fn delete(self, user: &dyn Person, store: &mut impl Store) -> TbResult<GarageId> {
        self.checkuser(user, store).await?;

        // Check if garage has any bikes
        let parts = store.garage_get_parts(self).await?;
        if !parts.is_empty() {
            return Err(Error::Conflict("Garage still has bikes assigned".into()));
        }

        store.garage_delete(self).await?;
        Ok(self)
    }

    /// Register a part (bike) to this garage
    /// Can be done by garage owner or any user with an active subscription
    /// Automatically registers all currently attached parts (cascading registration)
    /// Returns a Summary with the registered part and its attachments
    pub async fn register_part(
        self,
        part_id: PartId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<crate::Summary> {
        // Verify the part exists and user owns it
        part_id.checkuser(user, store).await?;

        // Check if user is garage owner OR has active subscription
        let garage = store.garage_get(self).await?;
        let is_owner = garage.owner == user.get_id();
        let has_subscription = self.has_subscription(user, store).await?;

        if !(is_owner || has_subscription) {
            return Err(Error::Forbidden(
                "You must subscribe to this garage before registering bikes".into(),
            ));
        }

        // Build summary to return
        let mut hash = crate::SumHash::default();

        // Register the part (bike or spare) to the garage
        store.garage_register_part(self, part_id).await?;

        // Add the registered part to the summary
        let part = part_id.read(store).await?;
        hash += part.clone();

        // For display purposes: if this is a main part (bike), include its current attachments in the response
        // Note: These attachments are NOT stored in garage_parts, they will be fetched dynamically later
        let (attachments, _) = crate::Attachment::for_part_with_usage(part_id, store).await?;
        let now = time::OffsetDateTime::now_utc();

        for attachment in attachments {
            // Only include currently attached parts (now < detached)
            if now < attachment.a.detached {
                // Add attached part to response summary (for UI display)
                if let Ok(attached_part) = attachment.a.part_id.read(store).await {
                    hash += attached_part;
                }

                // Add attachment detail to response summary
                hash += crate::Summary {
                    attachments: vec![attachment],
                    ..Default::default()
                };
            }
        }

        // Add usage for the registered part
        if let Ok(usage) = part.usage.read(store).await {
            hash += crate::Summary {
                usages: vec![usage],
                ..Default::default()
            };
        }

        Ok(hash.into())
    }

    async fn has_subscription(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> Result<bool, Error> {
        Ok(store
            .subscription_find_active(self, user.get_id())
            .await?
            .is_some())
    }

    /// Unregister a part (bike) from this garage
    /// Can be done by garage owner OR part owner
    /// Returns an empty Summary (for consistency with other endpoints)
    pub async fn unregister_part(
        self,
        part_id: PartId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<crate::Summary> {
        // Check if user is garage owner OR part owner
        let garage = store.garage_get(self).await?;
        let is_garage_owner = garage.owner == user.get_id();

        // Check if user owns the part
        let is_part_owner = if let Ok(part) = part_id.read(store).await {
            part.owner == user.get_id()
        } else {
            false
        };

        if !is_garage_owner && !is_part_owner {
            return Err(Error::Forbidden(
                "You must be the garage owner or part owner to unregister this part".into(),
            ));
        }

        store.garage_unregister_part(self, part_id).await?;

        // Return empty summary (part is removed, nothing to update)
        Ok(crate::Summary::default())
    }

    /// Get all parts registered to this garage
    /// Can be accessed by garage owner or users with active subscription
    pub async fn get_parts(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Vec<PartId>> {
        // Check if user is garage owner OR has active subscription
        let garage = store.garage_get(self).await?;
        let is_owner = garage.owner == user.get_id();
        let has_subscription = self.has_subscription(user, store).await?;

        if !is_owner && !has_subscription {
            return Err(Error::Forbidden(
                "You must be the garage owner or have an active subscription to view bikes".into(),
            ));
        }

        store.garage_get_parts(self).await
    }

    /// Get garage details with all related data (for garage mode)
    /// Returns parts, attachments, services, plans, and usages (NOT activities)
    pub async fn get_details(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<crate::GarageSummary> {
        // Check permissions (garage owner or active subscriber)
        let garage = store.garage_get(self).await?;
        let is_owner = garage.owner == user.get_id();
        let has_subscription = self.has_subscription(user, store).await?;

        if !is_owner && !has_subscription {
            return Err(Error::Forbidden(
                "You must be the garage owner or have an active subscription to view garage details".into(),
            ));
        }

        // Get all part IDs registered to this garage
        let part_ids = store.garage_get_parts(self).await?;

        // Fetch full part details
        let mut parts = Vec::new();
        for part_id in &part_ids {
            let part = part_id.read(store).await?;
            // Privacy filter: non-owners only see their own parts
            if is_owner || part.owner == user.get_id() {
                parts.push(part);
            }
        }

        // Get attachments - DYNAMIC
        let mut attachments = Vec::new();
        let now = time::OffsetDateTime::now_utc();
        for part in &mut parts {
            // Only fetch attachments for main parts (bikes/gears)
            let (part_attachments, _) =
                crate::Attachment::for_part_with_usage(part.id, store).await?;
            // Only include currently attached parts (now < detached)
            for attachment in part_attachments {
                if now < attachment.a.detached {
                    attachments.push(attachment);
                }
            }
        }

        for attachment in &attachments {
            parts.push(attachment.a.part_id.read(store).await?);
        }

        // Get services for these parts
        let mut services = Vec::new();
        for part in &parts {
            let (part_services, _) = crate::Service::for_part_with_usage(part.id, store).await?;
            services.extend(part_services);
        }

        // Get service plans for these parts
        let mut plans = Vec::new();
        for part in &parts {
            let part_plans = crate::ServicePlan::for_part(part.id, store).await?;
            plans.extend(part_plans);
        }

        // Get usages for these parts
        let mut usages = Vec::new();
        for part in &parts {
            if let Ok(usage) = part.usage.read(store).await {
                usages.push(usage);
            }
        }

        Ok(crate::GarageSummary {
            garage,
            parts,
            attachments,
            services,
            plans,
            usages,
        })
    }

    /// Read a garage from the database
    /// Allows access for owners, and active subscribers
    pub async fn read(self, user: &dyn Person, store: &mut impl Store) -> TbResult<Garage> {
        self.check_read_access(user, store).await?;
        store.garage_get(self).await
    }
}

impl Garage {
    /// Get all garages for a user
    pub async fn get_all_for_user(
        user: &UserId,
        store: &mut impl GarageStore,
    ) -> TbResult<Vec<Garage>> {
        store.garages_get_all_for_user(*user).await
    }

    /// Search for garages by name (for users to find garages to request registration)
    pub async fn search(query: &str, store: &mut impl GarageStore) -> TbResult<Vec<Garage>> {
        store.garages_search(query).await
    }

    /// Convert a list of garages to garages with owner information
    pub async fn with_owner_info(
        garages: Vec<Garage>,
        store: &mut impl Store,
    ) -> TbResult<Vec<GarageWithOwner>> {
        let mut result = Vec::new();
        for garage in garages {
            let owner = garage.owner.read(store).await?;
            result.push(GarageWithOwner::from_garage_and_user(garage, owner));
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

/// A subscription to a garage, allowing a user to register their bikes
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarageSubscription {
    pub id: SubscriptionId,
    pub garage_id: GarageId,
    pub user_id: UserId,
    pub status: SubscriptionStatus,
    pub message: Option<String>,
    pub response_message: Option<String>,
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde_as(as = "Rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// A subscription with garage details for API responses
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarageSubscriptionWithDetails {
    pub id: SubscriptionId,
    pub garage_id: GarageId,
    pub garage_name: String,
    pub garage_owner_firstname: String,
    pub garage_owner_name: String,
    pub user_id: UserId,
    pub status: SubscriptionStatus,
    pub message: Option<String>,
    pub response_message: Option<String>,
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde_as(as = "Rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl GarageSubscriptionWithDetails {
    /// Create from subscription and garage with owner
    pub fn from_subscription_and_garage(
        subscription: GarageSubscription,
        garage: GarageWithOwner,
    ) -> Self {
        Self {
            id: subscription.id,
            garage_id: subscription.garage_id,
            garage_name: garage.name,
            garage_owner_firstname: garage.owner_firstname,
            garage_owner_name: garage.owner_name,
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
        garage_id: GarageId,
        message: Option<String>,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageSubscription> {
        // Verify the garage exists (don't check ownership - users can subscribe to any garage)
        store.garage_get(garage_id).await?;

        // Check if there's already a pending subscription
        let existing = store
            .subscription_find_pending(garage_id, user.get_id())
            .await?;
        if existing.is_some() {
            return Err(Error::Conflict(
                "A pending subscription request already exists".into(),
            ));
        }

        // Check if there's already an active subscription
        let active = store
            .subscription_find_active(garage_id, user.get_id())
            .await?;
        if active.is_some() {
            return Err(Error::Conflict(
                "You are already subscribed to this garage".into(),
            ));
        }

        store
            .subscription_create(garage_id, user.get_id(), message)
            .await
    }

    /// Get a subscription by ID
    pub async fn get(
        id: i32,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<SubscriptionId> {
        SubscriptionId(id).checkuser(user, store).await
    }

    /// Read a subscription from the database
    pub async fn read(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageSubscription> {
        self.checkuser(user, store).await?;
        store.subscription_get(self).await
    }

    /// Check if the user has access to this subscription (either subscriber or garage owner)
    pub async fn checkuser(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<SubscriptionId> {
        let subscription = store.subscription_get(self).await?;

        // Allow access if user is the subscriber
        if subscription.user_id == user.get_id() {
            return Ok(self);
        }

        // Allow access if user owns the garage
        let garage = store.garage_get(subscription.garage_id).await?;
        user.check_owner(garage.owner, "Access denied to subscription".to_string())?;

        Ok(self)
    }

    /// Approve a subscription (garage owner only)
    pub async fn approve(
        self,
        response_message: Option<String>,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageSubscription> {
        let subscription = store.subscription_get(self).await?;

        // Verify user owns the garage
        let garage_id = subscription.garage_id;
        garage_id.checkuser(user, store).await?;

        if subscription.status != SubscriptionStatus::Pending {
            return Err(Error::Conflict("Subscription is not pending".into()));
        }

        // Update subscription status to active with response message
        store
            .subscription_approve(self, SubscriptionStatus::Active, response_message)
            .await
    }

    /// Reject a subscription (garage owner only)
    pub async fn reject(
        self,
        response_message: Option<String>,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageSubscription> {
        let subscription = store.subscription_get(self).await?;

        // Verify user owns the garage
        let garage_id = subscription.garage_id;
        garage_id.checkuser(user, store).await?;

        if subscription.status != SubscriptionStatus::Pending {
            return Err(Error::Conflict("Subscription is not pending".into()));
        }

        store
            .subscription_approve(self, SubscriptionStatus::Rejected, response_message)
            .await
    }

    /// Cancel a subscription (subscriber only)
    /// Allows deletion of pending, active, and rejected subscriptions
    pub async fn cancel(self, user: &dyn Person, store: &mut impl Store) -> TbResult<()> {
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

        store.subscription_delete(self).await
    }
}

impl GarageSubscription {
    /// Get all pending subscriptions for a garage (garage owner only)
    pub async fn get_pending_for_garage(
        garage_id: GarageId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Vec<GarageSubscription>> {
        garage_id.checkuser(user, store).await?;
        store
            .subscriptions_for_garage(garage_id, Some(SubscriptionStatus::Pending))
            .await
    }

    /// Get all subscriptions made by a user
    pub async fn get_for_user(
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Vec<GarageSubscription>> {
        store.subscriptions_for_user(user.get_id()).await
    }

    /// Convert a list of subscriptions to subscriptions with garage details
    pub async fn with_garage_details(
        subscriptions: Vec<GarageSubscription>,
        store: &mut impl Store,
    ) -> TbResult<Vec<GarageSubscriptionWithDetails>> {
        let mut result = Vec::new();
        for subscription in subscriptions {
            let garage = store.garage_get(subscription.garage_id).await?;
            let owner = garage.owner.read(store).await?;
            let garage_with_owner = GarageWithOwner::from_garage_and_user(garage, owner);
            result.push(GarageSubscriptionWithDetails::from_subscription_and_garage(
                subscription,
                garage_with_owner,
            ));
        }
        Ok(result)
    }
}
