use crate::{TbResult, Usage, UsageId};

#[async_trait::async_trait]
/// A trait representing a store for `Usage` objects.
pub trait UsageStore {
    /// Retrieves or creates a `Usage` by its ID.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the Usage to retrieve.
    ///
    /// # Returns
    ///
    /// Returns the Usage object if the Usage exists, None if not
    /// Might returns an error from underlying storage system.
    async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>>;

    /// Changes an array of `Usage` objects.
    /// Might delete the Usages on the store if it is all zero
    /// If a Usage does not exist, it will create it on the store
    ///
    /// # Arguments
    ///
    /// * `Usage` - The `Usage` object containing the changes to apply.
    ///
    /// # Returns
    ///
    /// Returns the updated `Usage` object if it was successfully updated, otherwise returns an error.
    async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize>;

    /// Delete the Usage
    ///
    /// # Arguments
    ///
    /// * `Usage` - The `Usage` object to delete.
    ///
    /// # Returns
    ///
    /// Returns the number of deleted objects or returns an error.
    async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage>;

    /// Deletes an array of usages
    ///
    /// # Arguments
    ///
    /// * `usages` - A Vector of usages to delete
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of deleted usages or an error if the operation fails.
    async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize>;

    /// Resets all Usages.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of reset parts or an error if the operation fails.
    async fn usage_delete_all(&mut self) -> TbResult<usize>;
}
