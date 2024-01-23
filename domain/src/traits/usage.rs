use crate::{TbResult, Usage, UsageId};

#[async_trait::async_trait]
/// A trait representing a store for `Usage` objects.
pub trait UsageStore {
    /// Retrieves a `Usage` by its ID.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the Usage to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Usage` object if the Usage exists, otherwise returns an error.
    async fn read(&mut self, uid: UsageId) -> TbResult<Usage>;

    /// Creates a new `Usage` object.
    ///
    /// # Arguments
    ///
    /// * `usage` - The new `Usage` object to create.
    ///
    /// # Returns
    ///
    /// Returns the newly created `Usage` object if it was successfully created, otherwise returns an error.
    async fn create(&mut self, usage: &Usage) -> TbResult<usize>;

    /// Changes an existing `Usage` object.
    ///
    /// # Arguments
    ///
    /// * `Usage` - The `Usage` object containing the changes to apply.
    ///
    /// # Returns
    ///
    /// Returns the updated `Usage` object if it was successfully updated, otherwise returns an error.
    async fn update(&mut self, usage: Vec<&Usage>) -> TbResult<usize>;
}
