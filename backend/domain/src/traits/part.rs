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
}
