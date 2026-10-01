use time::OffsetDateTime;

use crate::{ActTypeId, Activity, ActivityId, PartId, TbResult, UserId};

// A trait for storing and retrieving activities.
/// A trait defining the methods for storing and retrieving activities.
#[async_trait::async_trait]
pub trait ActivityStore {
    /// Creates a new activity.
    ///
    /// The activity id is a primary key: an id the store already holds
    /// fails the create with [`crate::Error::DatabaseFailure`] — the
    /// database's INSERT hits the key, and the in-memory store applies the
    /// same one rule (issue #405).
    ///
    /// # Arguments
    ///
    /// * `act` - A reference to a `Activity` struct containing the details of the new activity.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the newly created `Activity`, or an error if the
    /// operation fails (a duplicate id is a `DatabaseFailure`).
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity>;

    /// Retrieves an activity by its ID.
    ///
    /// # Arguments
    ///
    /// * `aid` - The ID of the activity to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the retrieved `Activity` or an error if the operation fails.
    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>>;

    /// Updates an existing activity.
    ///
    /// The data fields are replaced with the values of `act`: user, type,
    /// name, start, duration, time, distance, climb, descend, energy, and
    /// gear. The row keeps the three fields that are lost when the
    /// frontend round-trips an activity: the stored `utc_offset` (the new
    /// start is expressed in the stored offset), `device_name`, and
    /// `external_id` — both stores apply this one rule.
    ///
    /// # Arguments
    ///
    /// * `act` - The activity with the updated details.
    ///
    /// # Returns
    ///
    /// Returns the stored row after the update.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if no activity with this id exists.
    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity>;

    /// Deletes an activity by its ID.
    ///
    /// # Arguments
    ///
    /// * `aid` - The ID of the activity to delete.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of deleted activities or an error if the operation fails.
    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize>;

    /// Deletes an activity by its ID.
    ///
    /// # Arguments
    ///
    /// * `activities` - An Vector of activities to delete
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of deleted activities or an error if the operation fails.
    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize>;

    /// Retrieves all activities for a given user ID.
    ///
    /// Activities come back in ascending start instant (the database's
    /// `ORDER BY start`): two activities with the same start instant may
    /// come back in any order — both stores apply this one rule.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to retrieve activities for.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>>;

    /// Retrieves all activities for a given part ID and time range.
    ///
    /// An activity matches when `begin <= start < end` (start is the
    /// activity's start instant): `begin` is included, `end` is excluded —
    /// both stores apply this one rule.
    ///
    /// # Arguments
    ///
    /// * `part` - The ID of the part to retrieve activities for.
    /// * `begin` - The start of the time range (inclusive).
    /// * `end` - The end of the time range (exclusive).
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>>;

    /// Retrieves the user's activity that started in the minute of `rstart`.
    ///
    /// The match is by the minute, not the instant: the activity's start —
    /// in its stored offset, i.e. the user's local wall clock — floored to
    /// the minute must equal `rstart`'s minute (the database truncates the
    /// stored start to its local minute, adds the stored offset, and
    /// compares it to the truncated query time; both stores apply this one
    /// rule). `rstart` is the query time as an instant, e.g. the CSV
    /// import's local wall clock parsed as UTC.
    ///
    /// The match must be unambiguous (maintainer-confirmed, issue #408):
    /// zero matches returns [`Error::NotFound`], exactly one match returns
    /// that activity, and two or more activities of the user in the same
    /// minute returns [`Error::Ambiguous`] — a conflicting import row must
    /// fail loudly (the CSV path puts it in the bad list), never silently
    /// update one of the rides.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to retrieve the activity for.
    /// * `rstart` - The start time to match the activity's local minute against.
    ///
    /// # Returns
    ///
    /// Returns the matched `Activity`; [`Error::NotFound`] if none of the
    /// user's activities falls in the minute, [`Error::Ambiguous`] if more
    /// than one does.
    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity>;

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
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>>;

    /// Retrieves all activities.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `Activity` structs or an error if the operation fails.
    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>>;
}
