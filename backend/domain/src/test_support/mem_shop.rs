use super::*;
use crate::{Shop, ShopId, ShopSubscription, SubscriptionId, SubscriptionStatus, TbResult, UserId};

#[async_trait::async_trait]
impl ShopStore for MemStore {
    async fn shop_create(
        &mut self,
        _name: String,
        _description: Option<String>,
        _auto_approve: bool,
        _owner: UserId,
    ) -> TbResult<Shop> {
        todo!()
    }

    async fn shop_get(&mut self, _id: ShopId) -> TbResult<Shop> {
        todo!()
    }

    async fn shop_update(
        &mut self,
        _id: ShopId,
        _name: String,
        _description: Option<String>,
        _auto_approve: bool,
    ) -> TbResult<Shop> {
        todo!()
    }

    async fn shop_delete(&mut self, _id: ShopId) -> TbResult<usize> {
        todo!()
    }

    async fn shops_get_all_for_user(&mut self, _user_id: UserId) -> TbResult<Vec<Shop>> {
        todo!()
    }

    async fn shops_search(&mut self, _query: &str) -> TbResult<Vec<Shop>> {
        todo!()
    }

    async fn subscription_create(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
        _message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_get(&mut self, _id: SubscriptionId) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_find_active(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        todo!()
    }

    async fn subscription_find_pending(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        todo!()
    }

    async fn subscription_update_status(
        &mut self,
        _id: SubscriptionId,
        _status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_approve(
        &mut self,
        _id: SubscriptionId,
        _status: SubscriptionStatus,
        _response_message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        todo!()
    }

    async fn subscription_delete(&mut self, _id: SubscriptionId) -> TbResult<()> {
        todo!()
    }

    async fn subscriptions_for_shop(
        &mut self,
        _shop_id: ShopId,
    ) -> TbResult<Vec<ShopSubscription>> {
        todo!()
    }

    async fn subscriptions_for_user(
        &mut self,
        _user_id: UserId,
    ) -> TbResult<Vec<ShopSubscription>> {
        todo!()
    }
}
