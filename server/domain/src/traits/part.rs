use crate::{AnyResult, Part, PartId, PartTypeId, Person, Usage, UserId};
use time::OffsetDateTime;

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
    async fn partid_get_part(&mut self, pid: PartId) -> AnyResult<Part>;

    /// Retrieves the name of a `Part` by its ID.
    ///
    /// # Arguments
    ///
    /// * `pid` - The ID of the part to retrieve the name for.
    ///
    /// # Returns
    ///
    /// Returns the name of the `Part` if it exists, otherwise returns an error.
    async fn partid_get_name(&mut self, pid: PartId) -> AnyResult<String>;

    /// Retrieves the type of a `Part` by its ID.
    ///
    /// # Arguments
    ///
    /// * `pid` - The ID of the part to retrieve the type for.
    ///
    /// # Returns
    ///
    /// Returns the type of the `Part` if it exists, otherwise returns an error.
    async fn partid_get_type(&mut self, pid: PartId) -> AnyResult<PartTypeId>;

    /// Retrieves the owner ID of a `Part` by its ID and the user requesting the information.
    ///
    /// # Arguments
    ///
    /// * `pid` - The ID of the part to retrieve the owner ID for.
    /// * `user` - The user requesting the information.
    ///
    /// # Returns
    ///
    /// Returns the owner ID of the `Part` if it exists and the user has permission to access it, otherwise returns an error.
    async fn partid_get_ownerid(&mut self, pid: PartId, user: &dyn Person) -> AnyResult<UserId>;

    /// Applies usage to a `Part` by its ID.
    ///
    /// # Arguments
    ///
    /// * `pid` - The ID of the part to apply usage to.
    /// * `usage` - The usage to apply to the part.
    /// * `start` - The start time of the usage.
    ///
    /// # Returns
    ///
    /// Returns the updated `Part` object if the part exists and the usage was successfully applied, otherwise returns an error.
    async fn partid_apply_usage(
        &mut self,
        pid: PartId,
        usage: &Usage,
        start: OffsetDateTime,
    ) -> AnyResult<Part>;

    /// Retrieves all `Part` objects for a given user ID.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to retrieve parts for.
    ///
    /// # Returns
    ///
    /// Returns a vector of `Part` objects if the user has parts, otherwise returns an error.
    async fn part_get_all_for_userid(&mut self, uid: UserId) -> AnyResult<Vec<Part>>;

    /// Resets all usages for a given user's `Part` objects.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to reset usages for.
    ///
    /// # Returns
    ///
    /// Returns a vector of updated `Part` objects if the user has parts and the usages were successfully reset, otherwise returns an error.
    async fn parts_reset_all_usages(&mut self, uid: UserId) -> AnyResult<Vec<Part>>;

    /// Creates a new `Part` object.
    ///
    /// # Arguments
    ///
    /// * `newpart` - The new `Part` object to create.
    /// * `createtime` - The time the part was created.
    ///
    /// # Returns
    ///
    /// Returns the newly created `Part` object if it was successfully created, otherwise returns an error.
    async fn create_part(
        &mut self,
        newpart: crate::NewPart,
        createtime: OffsetDateTime,
    ) -> AnyResult<Part>;

    /// Changes an existing `Part` object.
    ///
    /// # Arguments
    ///
    /// * `part` - The `ChangePart` object containing the changes to apply to the `Part`.
    ///
    /// # Returns
    ///
    /// Returns the updated `Part` object if it was successfully updated, otherwise returns an error.
    async fn part_change(&mut self, part: crate::ChangePart) -> AnyResult<Part>;
}
