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
    /// Returns the `Usage` object if the Usage exists, an empty new one if not
    /// Might returns an error from underlying storage system.
    async fn usage_get(&mut self, uid: UsageId) -> TbResult<Usage>;

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
    async fn usage_update<U>(&mut self, usage: &Vec<U>) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync;

    /// Delete the Usage
    ///
    /// # Arguments
    ///
    /// * `Usage` - The `Usage` object to delete.
    ///
    /// # Returns
    ///
    /// Returns the number of deleted objects or returns an error.
    async fn usage_delete(&mut self, usage: &UsageId) -> TbResult<Usage>;

    /// Resets all Usages.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of reset parts or an error if the operation fails.
    async fn usage_delete_all(&mut self) -> TbResult<usize>;
}
