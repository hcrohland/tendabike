use crate::{Shop, ShopId, TbResult, UserId};

#[async_trait::async_trait]
/// A trait representing a shop store.
pub trait ShopStore {
    /// Creates a new shop.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the shop.
    /// * `description` - Optional description of the shop.
    /// * `owner` - The user ID of the shop owner.
    ///
    /// # Returns
    ///
    /// The newly created shop.
    async fn shop_create(
        &mut self,
        name: String,
        description: Option<String>,
        owner: UserId,
    ) -> TbResult<Shop>;

    /// Reads a shop by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shop to read.
    ///
    /// # Returns
    ///
    /// The shop with the given ID, if it exists.
    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop>;

    /// Updates an existing shop.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shop to update.
    /// * `name` - The new name of the shop.
    /// * `description` - The new description of the shop.
    ///
    /// # Returns
    ///
    /// The updated shop.
    async fn shop_update(
        &mut self,
        id: ShopId,
        name: String,
        description: Option<String>,
    ) -> TbResult<Shop>;

    /// Deletes a shop.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shop to delete.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing 1 or an error if the operation fails.
    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize>;

    /// Gets all shops for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user.
    ///
    /// # Returns
    ///
    /// A vector of shops owned by the user.
    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>>;

    /// Searches for shops by name.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string.
    ///
    /// # Returns
    ///
    /// A vector of shops matching the search query.
    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>>;

    // Subscription methods

    /// Creates a new subscription request.
    async fn subscription_create(
        &mut self,
        shop_id: crate::ShopId,
        user_id: UserId,
        message: Option<String>,
    ) -> TbResult<crate::ShopSubscription>;

    /// Gets a subscription by ID.
    async fn subscription_get(
        &mut self,
        id: crate::SubscriptionId,
    ) -> TbResult<crate::ShopSubscription>;

    /// Finds an active subscription for a specific shop and user.
    async fn subscription_find_active(
        &mut self,
        shop_id: crate::ShopId,
        user_id: UserId,
    ) -> TbResult<Option<crate::ShopSubscription>>;

    /// Finds a pending subscription for a specific shop and user.
    async fn subscription_find_pending(
        &mut self,
        shop_id: crate::ShopId,
        user_id: UserId,
    ) -> TbResult<Option<crate::ShopSubscription>>;

    /// Updates the status of a subscription.
    async fn subscription_update_status(
        &mut self,
        id: crate::SubscriptionId,
        status: crate::SubscriptionStatus,
    ) -> TbResult<crate::ShopSubscription>;

    /// Approves or rejects a subscription with an optional response message.
    async fn subscription_approve(
        &mut self,
        id: crate::SubscriptionId,
        status: crate::SubscriptionStatus,
        response_message: Option<String>,
    ) -> TbResult<crate::ShopSubscription>;

    /// Deletes a subscription.
    async fn subscription_delete(&mut self, id: crate::SubscriptionId) -> TbResult<()>;

    /// Gets all subscriptions for a shop, optionally filtered by status.
    async fn subscriptions_for_shop(
        &mut self,
        shop_id: crate::ShopId,
        status: Option<crate::SubscriptionStatus>,
    ) -> TbResult<Vec<crate::ShopSubscription>>;

    /// Gets all subscriptions for a user.
    async fn subscriptions_for_user(
        &mut self,
        user_id: UserId,
    ) -> TbResult<Vec<crate::ShopSubscription>>;
}
