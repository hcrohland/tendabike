use crate::{Error, Shop, Store, TbResult, UserId};

use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::ShopId;

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
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShopSubscriptionWithDetails {
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
    pub shop: Shop,
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

#[derive(Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionId(i32);

impl SubscriptionId {
    /// Create a new subscription request
    pub async fn create(
        shop_id: ShopId,
        message: Option<String>,
        user: UserId,
        store: &mut impl Store,
    ) -> TbResult<ShopSubscription> {
        // Verify the shop exists (don't check ownership - users can subscribe to any shop)
        let shop = store.shop_get(shop_id).await?;

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

        let subscription = store.subscription_create(shop_id, user, message).await?;
        if shop.auto_approve {
            // Update subscription status to active with response message
            store
                .subscription_approve(
                    subscription.id,
                    SubscriptionStatus::Active,
                    Some(format!("Welcome to {}", shop.name)),
                )
                .await
        } else {
            Ok(subscription)
        }
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
        shop_id.check_owner(user, store).await?;

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
        shop_id.check_owner(user, store).await?;

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
    ) -> TbResult<Vec<ShopSubscriptionWithDetails>> {
        let shop = shop_id.check_owner(user, store).await?;
        Ok(store
            .subscriptions_for_shop(shop_id)
            .await?
            .into_iter()
            .filter(|s| s.status == SubscriptionStatus::Pending)
            .map(|s| s.add_shop(shop.clone()))
            .collect())
    }

    pub async fn get_for_shop(
        shop_id: ShopId,
        store: &mut impl Store,
    ) -> TbResult<Vec<ShopSubscription>> {
        store.subscriptions_for_shop(shop_id).await
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
            result.push(subscription.add_shop(shop));
        }
        Ok(result)
    }

    pub(super) async fn check(shop: ShopId, user: UserId, store: &mut impl Store) -> TbResult<()> {
        let subs = store.subscriptions_for_user(user).await?;
        match subs.into_iter().find(|s| s.shop_id == shop) {
            Some(s) if s.status == SubscriptionStatus::Active => Ok(()),
            _ => Err(Error::Forbidden(
                "You are not subscribed to this shop".to_string(),
            )),
        }
    }

    pub fn add_shop(self, shop: Shop) -> ShopSubscriptionWithDetails {
        ShopSubscriptionWithDetails {
            id: self.id,
            shop_id: self.shop_id,
            user_id: self.user_id,
            status: self.status,
            message: self.message,
            response_message: self.response_message,
            created_at: self.created_at,
            updated_at: self.updated_at,
            shop,
        }
    }
}
