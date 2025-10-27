#![allow(clippy::too_many_arguments)]
use time::OffsetDateTime;

use crate::{Part, PartId, PartTypeId, TbResult, UsageId, UserId};

#[async_trait::async_trait]
/// A trait representing a store for `Part` objects.
pub trait PartStore {
    /// Retrieves a `Part` by its ID.
    ///
    /// # Arguments
    ///
    /// * `pid` - The ID of the part to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Part` object if the part exists, otherwise returns an error.
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part>;

    /// Retrieves all `Part` objects for a given user ID.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to retrieve parts for.
    ///
    /// # Returns
    ///
    /// Returns a vector of `Part` objects if the user has parts, otherwise returns an error.
    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>>;

    /// Creates a new `Part` object.
    ///
    /// # Arguments
    ///
    /// * `newpart` - The new `Part` object to create.
    ///
    /// # Returns
    ///
    /// Returns the newly created `Part` object if it was successfully created, otherwise returns an error.
    async fn part_create(
        &mut self,
        what: PartTypeId,
        name: String,
        vendor: String,
        model: String,
        purchase: OffsetDateTime,
        source: Option<String>,
        usage: UsageId,
        owner: UserId,
    ) -> TbResult<Part>;

    /// updates an existing part
    ///
    /// # Errors
    ///
    /// This function will return an error if the part does not exist or there is a store error.
    async fn part_update(&mut self, part: Part) -> TbResult<Part>;

    /// updates an existing part
    ///
    /// # Arguments
    ///
    /// This function will return an error if the part does not exist or there is a store error.
    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId>;

    /// Deletes an array of parts
    ///
    /// # Arguments
    ///
    /// * `parts` - A Vector of parts to delete
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of deleted parts or an error if the operation fails.
    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize>;

    /// Returns the PartId associated with the given Strava gear ID.
    ///
    /// # Arguments
    ///
    /// * `strava_id` - The Strava ID of the gear.
    ///
    /// # Returns
    ///
    /// The PartId associated with the given Strava gear ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the PartId cannot be retrieved from the database.
    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>>;
}
