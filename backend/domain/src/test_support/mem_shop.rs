use super::*;
use crate::{Shop, ShopId, ShopSubscription, SubscriptionId, SubscriptionStatus, TbResult, UserId};

#[async_trait::async_trait]
impl ShopStore for MemStore {
    async fn shop_create(
        &mut self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        owner: UserId,
    ) -> TbResult<Shop> {
        todo!()
    }

    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop> {
        todo!()
    }

    async fn shop_update(
        &mut self,
        id: ShopId,
        name: String,
        description: Option<String>,
        auto_approve: bool,
    ) -> TbResult<Shop> {
        todo!()
    }

    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize> {
        todo!()
    }

    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>> {
        todo!()
    }

    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>> {
        todo!()
    }

    async fn subscription_create(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
        message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_find_active(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        todo!()
    }

    async fn subscription_find_pending(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        todo!()
    }

    async fn subscription_update_status(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_approve(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
        response_message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()> {
        todo!()
    }

    async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>> {
        todo!()
    }

    async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>> {
        todo!()
    }
}
