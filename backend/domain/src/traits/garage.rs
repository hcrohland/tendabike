use crate::{Garage, GarageId, TbResult, UserId};

#[async_trait::async_trait]
/// A trait representing a garage store.
pub trait GarageStore {
    /// Creates a new garage.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the garage.
    /// * `description` - Optional description of the garage.
    /// * `owner` - The user ID of the garage owner.
    ///
    /// # Returns
    ///
    /// The newly created garage.
    async fn garage_create(
        &mut self,
        name: String,
        description: Option<String>,
        owner: UserId,
    ) -> TbResult<Garage>;

    /// Reads a garage by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the garage to read.
    ///
    /// # Returns
    ///
    /// The garage with the given ID, if it exists.
    async fn garage_get(&mut self, id: GarageId) -> TbResult<Garage>;

    /// Updates an existing garage.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the garage to update.
    /// * `name` - The new name of the garage.
    /// * `description` - The new description of the garage.
    ///
    /// # Returns
    ///
    /// The updated garage.
    async fn garage_update(
        &mut self,
        id: GarageId,
        name: String,
        description: Option<String>,
    ) -> TbResult<Garage>;

    /// Deletes a garage.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the garage to delete.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing 1 or an error if the operation fails.
    async fn garage_delete(&mut self, id: GarageId) -> TbResult<usize>;

    /// Gets all garages for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user.
    ///
    /// # Returns
    ///
    /// A vector of garages owned by the user.
    async fn garages_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Garage>>;

    /// Registers a part (bike) to a garage.
    ///
    /// # Arguments
    ///
    /// * `garage_id` - The ID of the garage.
    /// * `part_id` - The ID of the part to register.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if successful, error otherwise.
    async fn garage_register_part(
        &mut self,
        garage_id: crate::GarageId,
        part_id: crate::PartId,
    ) -> TbResult<()>;

    /// Unregisters a part (bike) from a garage.
    ///
    /// # Arguments
    ///
    /// * `garage_id` - The ID of the garage.
    /// * `part_id` - The ID of the part to unregister.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if successful, error otherwise.
    async fn garage_unregister_part(
        &mut self,
        garage_id: crate::GarageId,
        part_id: crate::PartId,
    ) -> TbResult<()>;

    /// Gets all part IDs registered to a garage.
    ///
    /// # Arguments
    ///
    /// * `garage_id` - The ID of the garage.
    ///
    /// # Returns
    ///
    /// A vector of PartIds registered to the garage.
    async fn garage_get_parts(
        &mut self,
        garage_id: crate::GarageId,
    ) -> TbResult<Vec<crate::PartId>>;

    /// Gets the garage ID that a part is registered to, if any.
    ///
    /// # Arguments
    ///
    /// * `part_id` - The ID of the part.
    ///
    /// # Returns
    ///
    /// Optional GarageId if the part is registered to a garage.
    async fn part_get_garage(
        &mut self,
        part_id: crate::PartId,
    ) -> TbResult<Option<crate::GarageId>>;

    /// Searches for garages by name.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string.
    ///
    /// # Returns
    ///
    /// A vector of garages matching the search query.
    async fn garages_search(&mut self, query: &str) -> TbResult<Vec<Garage>>;

    // Subscription methods

    /// Creates a new subscription request.
    async fn subscription_create(
        &mut self,
        garage_id: crate::GarageId,
        user_id: UserId,
        message: Option<String>,
    ) -> TbResult<crate::GarageSubscription>;

    /// Gets a subscription by ID.
    async fn subscription_get(
        &mut self,
        id: crate::SubscriptionId,
    ) -> TbResult<crate::GarageSubscription>;

    /// Finds an active subscription for a specific garage and user.
    async fn subscription_find_active(
        &mut self,
        garage_id: crate::GarageId,
        user_id: UserId,
    ) -> TbResult<Option<crate::GarageSubscription>>;

    /// Finds a pending subscription for a specific garage and user.
    async fn subscription_find_pending(
        &mut self,
        garage_id: crate::GarageId,
        user_id: UserId,
    ) -> TbResult<Option<crate::GarageSubscription>>;

    /// Updates the status of a subscription.
    async fn subscription_update_status(
        &mut self,
        id: crate::SubscriptionId,
        status: crate::SubscriptionStatus,
    ) -> TbResult<crate::GarageSubscription>;

    /// Approves or rejects a subscription with an optional response message.
    async fn subscription_approve(
        &mut self,
        id: crate::SubscriptionId,
        status: crate::SubscriptionStatus,
        response_message: Option<String>,
    ) -> TbResult<crate::GarageSubscription>;

    /// Deletes a subscription.
    async fn subscription_delete(&mut self, id: crate::SubscriptionId) -> TbResult<()>;

    /// Gets all subscriptions for a garage, optionally filtered by status.
    async fn subscriptions_for_garage(
        &mut self,
        garage_id: crate::GarageId,
        status: Option<crate::SubscriptionStatus>,
    ) -> TbResult<Vec<crate::GarageSubscription>>;

    /// Gets all subscriptions for a user.
    async fn subscriptions_for_user(
        &mut self,
        user_id: UserId,
    ) -> TbResult<Vec<crate::GarageSubscription>>;
}
