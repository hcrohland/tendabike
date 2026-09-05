use super::*;
use crate::{
    Error, Shop, ShopId, ShopSubscription, SubscriptionId, SubscriptionStatus, TbResult, UserId,
};

#[async_trait::async_trait]
impl ShopStore for MemStore {
    async fn shop_create(
        &mut self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        owner: UserId,
    ) -> TbResult<Shop> {
        let id = ShopId::from(self.next_shop_id);
        self.next_shop_id += 1;
        let now = time::OffsetDateTime::now_utc();
        let shop = Shop {
            id,
            owner,
            name,
            description,
            auto_approve,
            created_at: now,
        };
        self.shops.insert(id, shop.clone());
        Ok(shop)
    }

    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop> {
        self.shops
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Shop {} not found", id)))
    }

    async fn shop_update(
        &mut self,
        id: ShopId,
        name: String,
        description: Option<String>,
        auto_approve: bool,
    ) -> TbResult<Shop> {
        match self.shops.get_mut(&id) {
            Some(shop) => {
                shop.name = name;
                shop.description = description;
                shop.auto_approve = auto_approve;
                Ok(shop.clone())
            }
            None => Err(Error::NotFound(format!("Shop {} not found", id))),
        }
    }

    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize> {
        if self.shops.remove(&id).is_some() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>> {
        let mut result: Vec<Shop> = self
            .shops
            .values()
            .filter(|s| {
                s.owner == user_id
                    || self.subscriptions.values().any(|sub| {
                        sub.shop_id == s.id
                            && sub.user_id == user_id
                            && sub.status == SubscriptionStatus::Active
                    })
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    // Simplified vs SQL: matches shop name only (SQL also matches owner
    // firstname/lastname/combinations and applies LIMIT 50).
    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>> {
        let q = query.to_lowercase();
        let mut result: Vec<Shop> = self
            .shops
            .values()
            .filter(|s| s.name.to_lowercase().contains(&q))
            .cloned()
            .collect();
        result.sort_by_key(|a| a.name.clone());
        Ok(result)
    }

    async fn subscription_create(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
        message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        let id = SubscriptionId::from(self.next_subscription_id);
        self.next_subscription_id += 1;
        let now = time::OffsetDateTime::now_utc();
        let sub = ShopSubscription {
            id,
            shop_id,
            user_id,
            status: SubscriptionStatus::Pending,
            message,
            response_message: None,
            created_at: now,
            updated_at: now,
        };
        self.subscriptions.insert(id, sub.clone());
        Ok(sub)
    }

    async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription> {
        self.subscriptions
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Subscription {} not found", id)))
    }

    async fn subscription_find_active(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        Ok(self
            .subscriptions
            .values()
            .find(|s| {
                s.shop_id == shop_id
                    && s.user_id == user_id
                    && s.status == SubscriptionStatus::Active
            })
            .cloned())
    }

    async fn subscription_find_pending(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        Ok(self
            .subscriptions
            .values()
            .find(|s| {
                s.shop_id == shop_id
                    && s.user_id == user_id
                    && s.status == SubscriptionStatus::Pending
            })
            .cloned())
    }

    async fn subscription_update_status(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        match self.subscriptions.get_mut(&id) {
            Some(sub) => {
                sub.status = status;
                sub.updated_at = time::OffsetDateTime::now_utc();
                Ok(sub.clone())
            }
            None => Err(Error::NotFound(format!("Subscription {} not found", id))),
        }
    }

    async fn subscription_approve(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
        response_message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        match self.subscriptions.get_mut(&id) {
            Some(sub) => {
                sub.status = status;
                sub.response_message = response_message;
                sub.updated_at = time::OffsetDateTime::now_utc();
                Ok(sub.clone())
            }
            None => Err(Error::NotFound(format!("Subscription {} not found", id))),
        }
    }

    async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()> {
        self.subscriptions.remove(&id);
        Ok(())
    }

    async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>> {
        let mut result: Vec<ShopSubscription> = self
            .subscriptions
            .values()
            .filter(|s| s.shop_id == shop_id)
            .cloned()
            .collect();
        result.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(result)
    }

    async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>> {
        let mut result: Vec<ShopSubscription> = self
            .subscriptions
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect();
        result.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(result)
    }
}
