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
    async fn usage_update(&mut self, usage: Vec<&Usage>) -> TbResult<usize>;

    /// Resets all Usages.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of reset parts or an error if the operation fails.
    async fn usage_reset_all(&mut self) -> TbResult<usize>;
}
