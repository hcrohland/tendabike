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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PartStore;
    use crate::ShopStore;
    use crate::test_support::MemStore;
    use crate::{PartTypeId, UsageId};

    async fn setup() -> (MemStore, UserId, UserId, Shop) {
        let mut store = MemStore::new();
        let owner = UserId::create("Shop", "Owner", &None, &mut store)
            .await
            .unwrap();
        let subscriber = UserId::create("Ride", "Fan", &None, &mut store)
            .await
            .unwrap();
        let shop = ShopId::create("Bike Barn".into(), None, false, owner, &mut store)
            .await
            .unwrap();
        (store, owner, subscriber, shop)
    }

    async fn setup_auto_approve() -> (MemStore, UserId, UserId, Shop) {
        let mut store = MemStore::new();
        let owner = UserId::create("Shop", "Owner", &None, &mut store)
            .await
            .unwrap();
        let subscriber = UserId::create("Ride", "Fan", &None, &mut store)
            .await
            .unwrap();
        let shop = ShopId::create("Auto Shop".into(), None, true, owner, &mut store)
            .await
            .unwrap();
        (store, owner, subscriber, shop)
    }

    // === Create ===

    #[tokio::test]
    async fn subscription_create_fresh_pending() {
        let (mut store, _, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, Some("Hi!".into()), subscriber, &mut store)
            .await
            .unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Pending);
        assert_eq!(sub.shop_id, shop.id);
        assert_eq!(sub.user_id, subscriber);
        assert_eq!(sub.message, Some("Hi!".into()));
    }

    #[tokio::test]
    async fn subscription_create_auto_approve_active() {
        let (mut store, _, subscriber, shop) = setup_auto_approve().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.response_message.is_some());
    }

    #[tokio::test]
    async fn subscription_create_dup_pending_conflict() {
        let (mut store, _, subscriber, shop) = setup().await;
        SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        let result = SubscriptionId::create(shop.id, None, subscriber, &mut store).await;
        assert!(matches!(result, Err(Error::Conflict(_))));
    }

    #[tokio::test]
    async fn subscription_create_dup_active_conflict() {
        let (mut store, _, subscriber, shop) = setup_auto_approve().await;
        SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        let result = SubscriptionId::create(shop.id, None, subscriber, &mut store).await;
        assert!(matches!(result, Err(Error::Conflict(_))));
    }

    // === checkuser ===

    #[tokio::test]
    async fn subscription_checkuser_subscriber_ok() {
        let (mut store, _, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        let result = sub.id.checkuser(subscriber, &mut store).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn subscription_checkuser_shop_owner_ok() {
        let (mut store, owner, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        let result = sub.id.checkuser(owner, &mut store).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn subscription_checkuser_stranger_forbidden() {
        let (mut store, owner, _, shop) = setup().await;
        let stranger = UserId::create("Eve", "X", &None, &mut store).await.unwrap();
        let sub = SubscriptionId::create(shop.id, None, owner, &mut store)
            .await
            .unwrap();
        let result = sub.id.checkuser(stranger, &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
    }

    // === approve / reject ===

    #[tokio::test]
    async fn subscription_approve_owner_only() {
        let (mut store, owner, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();

        // Non-owner cannot approve
        let result = sub.id.approve(None, subscriber, &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));

        // Owner can approve
        let approved = sub
            .id
            .approve(Some("Welcome!".to_string()), owner, &mut store)
            .await
            .unwrap();
        assert_eq!(approved.status, SubscriptionStatus::Active);
        assert_eq!(approved.response_message, Some("Welcome!".into()));
    }

    #[tokio::test]
    async fn subscription_approve_pending_only() {
        let (mut store, owner, _, shop) = setup_auto_approve().await;
        let sub = SubscriptionId::create(shop.id, None, owner, &mut store)
            .await
            .unwrap();
        // Already active (auto-approved)
        let result = sub.id.approve(None, owner, &mut store).await;
        assert!(matches!(result, Err(Error::Conflict(_))));
    }

    #[tokio::test]
    async fn subscription_reject_owner_only() {
        let (mut store, owner, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();

        let rejected = sub
            .id
            .reject(Some("Nope".to_string()), owner, &mut store)
            .await
            .unwrap();
        assert_eq!(rejected.status, SubscriptionStatus::Rejected);
        assert_eq!(rejected.response_message, Some("Nope".into()));
    }

    #[tokio::test]
    async fn subscription_reject_non_owner_forbidden() {
        let (mut store, _, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        let result = sub.id.reject(None, subscriber, &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
    }

    // === cancel ===

    #[tokio::test]
    async fn subscription_cancel_subscriber_only() {
        let (mut store, owner, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();

        // Shop owner cannot cancel
        let result = sub.id.cancel(owner, &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));

        // Subscriber can cancel
        sub.id.cancel(subscriber, &mut store).await.unwrap();
        assert!(sub.id.read(subscriber, &mut store).await.is_err());
    }

    #[tokio::test]
    async fn subscription_cancel_active_ok() {
        let (mut store, _, subscriber, shop) = setup_auto_approve().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Active);
        sub.id.cancel(subscriber, &mut store).await.unwrap();
    }

    #[tokio::test]
    async fn subscription_cancel_conflict_parts_in_shop() {
        let (mut store, _, subscriber, shop) = setup_auto_approve().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Active);

        // Register a part owned by subscriber in the shop
        let now = OffsetDateTime::now_utc();
        store
            .part_create(
                PartTypeId::from(1),
                "Test Bike".into(),
                "Trek".into(),
                "Marlin".into(),
                now,
                None,
                String::new(),
                UsageId::new(),
                subscriber,
                Some(shop.id),
            )
            .await
            .unwrap();

        let result = sub.id.cancel(subscriber, &mut store).await;
        assert!(matches!(&result, Err(Error::Conflict(msg)) if msg.contains("parts in the shop")));
    }

    // === queries ===

    #[tokio::test]
    async fn subscription_get_pending_for_shop() {
        let (mut store, owner, subscriber, shop) = setup().await;
        let s1 = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        // Make it active so it doesn't show in pending
        store
            .subscription_approve(s1.id, SubscriptionStatus::Active, None)
            .await
            .unwrap();

        let s2_user = UserId::create("U2", "L2", &None, &mut store).await.unwrap();
        SubscriptionId::create(shop.id, None, s2_user, &mut store)
            .await
            .unwrap();

        let pending = ShopSubscription::get_pending_for_shop(shop.id, owner, &mut store)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].user_id, s2_user);
        assert_eq!(pending[0].shop.name, "Bike Barn");
    }

    #[tokio::test]
    async fn subscription_get_for_user() {
        let (mut store, _, subscriber, shop) = setup().await;
        SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();

        let subs = ShopSubscription::get_for_user(subscriber, &mut store)
            .await
            .unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].shop_id, shop.id);
    }

    #[tokio::test]
    async fn subscription_with_shop_details() {
        let (mut store, _, subscriber, shop) = setup().await;
        let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();

        let details = ShopSubscription::with_shop_details(vec![sub], &mut store)
            .await
            .unwrap();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].shop.id, shop.id);
        assert_eq!(details[0].shop.name, "Bike Barn");
    }

    #[tokio::test]
    async fn subscription_check_active_ok() {
        let (mut store, _, subscriber, shop) = setup_auto_approve().await;
        SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        // check is pub(super), call via get_for_read which uses it internally
        ShopId::get_for_read(shop.id.into(), subscriber, &mut store)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn subscription_get_for_shop() {
        let (mut store, _, subscriber, shop) = setup().await;
        let _s1 = SubscriptionId::create(shop.id, None, subscriber, &mut store)
            .await
            .unwrap();
        let s2_user = UserId::create("U2", "L2", &None, &mut store).await.unwrap();
        SubscriptionId::create(shop.id, None, s2_user, &mut store)
            .await
            .unwrap();

        let subs = ShopSubscription::get_for_shop(shop.id, &mut store)
            .await
            .unwrap();
        assert_eq!(subs.len(), 2);
    }
}
