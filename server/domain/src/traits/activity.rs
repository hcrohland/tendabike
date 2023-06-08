use time::OffsetDateTime;

use crate::{ActTypeId, Activity, ActivityId, AnyResult, NewActivity, PartId, Person, UserId};
// A trait for storing and retrieving activities.
/// A trait defining the methods for storing and retrieving activities.
#[async_trait::async_trait]
pub trait ActivityStore {
    /// Creates a new activity.
    ///
    /// # Arguments
    ///
    /// * `act` - A reference to a `NewActivity` struct containing the details of the new activity.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the newly created `Activity` or an error if the operation fails.
    async fn activity_create(&mut self, act: &NewActivity) -> AnyResult<Activity>;

    /// Retrieves an activity by its ID.
    ///
    /// # Arguments
    ///
    /// * `aid` - The ID of the activity to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the retrieved `Activity` or an error if the operation fails.
    async fn activity_read_by_id(&mut self, aid: ActivityId) -> AnyResult<Activity>;

    /// Updates an existing activity.
    ///
    /// # Arguments
    ///
    /// * `aid` - The ID of the activity to update.
    /// * `act` - A reference to a `NewActivity` struct containing the updated details of the activity.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the updated `Activity` or an error if the operation fails.
    async fn activity_update(&mut self, aid: ActivityId, act: &NewActivity) -> AnyResult<Activity>;

    /// Deletes an activity by its ID.
    ///
    /// # Arguments
    ///
    /// * `aid` - The ID of the activity to delete.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of deleted activities or an error if the operation fails.
    async fn activity_delete(&mut self, aid: ActivityId) -> AnyResult<usize>;

    /// Retrieves all activities for a given user ID.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to retrieve activities for.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn activity_get_all_for_userid(&mut self, uid: UserId) -> AnyResult<Vec<Activity>>;

    /// Retrieves all activities for a given part ID and time range.
    ///
    /// # Arguments
    ///
    /// * `part` - The ID of the part to retrieve activities for.
    /// * `begin` - The start of the time range to retrieve activities for.
    /// * `end` - The end of the time range to retrieve activities for.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn activities_find_by_partid_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> AnyResult<Vec<Activity>>;

    /// Retrieves an activity for a given user ID and start time.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to retrieve the activity for.
    /// * `rstart` - The start time of the activity to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the retrieved `Activity` or an error if the operation fails.
    async fn activity_get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> AnyResult<Activity>;

    /// Sets the gear for an activity if it is null.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to a `Person` struct representing the user to set the gear for.
    /// * `types` - A vector of `ActTypeId` structs representing the types of activities to set the gear for.
    /// * `partid` - The ID of the part to set the gear for.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn activity_set_gear_if_null(
        &mut self,
        user: &dyn Person,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> AnyResult<Vec<Activity>>;

    /// Resets all parts.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of reset parts or an error if the operation fails.
    async fn part_reset_all(&mut self) -> AnyResult<usize>;

    /// Retrieves all activities.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn activity_get_really_all(&mut self) -> AnyResult<Vec<Activity>>;
}
