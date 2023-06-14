/*
This file defines the StravaStore trait, which extends the tb_domain::traits::Store trait
and provides additional methods for interacting with Strava data.
*/

use crate::{event::Event, StravaId, StravaUser};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use tb_domain::{ActivityId, AnyResult, PartId, Person, UserId};

#[async_trait]
pub trait StravaStore: tb_domain::Store + Send {
    /// Returns the user ID associated with the given Strava ID.
    ///
    /// # Arguments
    ///
    /// * `who` - The Strava ID of the user.
    ///
    /// # Returns
    ///
    /// The user ID associated with the given Strava ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the user ID cannot be retrieved from the database.
    async fn stravaid_get_user_id(&mut self, who: i32) -> AnyResult<i32>;

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
    async fn strava_gear_get_tbid(&mut self, strava_id: &str) -> AnyResult<Option<PartId>>;

    /// Returns the name of the gear associated with the given Strava gear ID.
    ///
    /// # Arguments
    ///
    /// * `gear` - The Strava ID of the gear.
    ///
    /// # Returns
    ///
    /// The name of the gear associated with the given Strava gear ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the name cannot be retrieved from the database.
    async fn strava_gearid_get_name(&mut self, gear: i32) -> AnyResult<String>;

    /// Returns the ActivityId associated with the given Strava activity ID.
    ///
    /// # Arguments
    ///
    /// * `strava_id` - The Strava ID of the activity.
    ///
    /// # Returns
    ///
    /// The ActivityId associated with the given Strava activity ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the ActivityId cannot be retrieved from the database.
    async fn strava_activity_get_tbid(&mut self, strava_id: i64) -> AnyResult<Option<ActivityId>>;

    /// Creates a new Strava activity with the given Strava ID and user ID.
    ///
    /// # Arguments
    ///
    /// * `strava_id` - The Strava ID of the activity.
    /// * `uid` - The user ID associated with the activity.
    /// * `new_id` - The new ActivityId to assign to the activity.
    ///
    /// # Errors
    ///
    /// Returns an error if the activity cannot be created.
    async fn strava_activity_new(
        &mut self,
        strava_id: i64,
        uid: UserId,
        new_id: ActivityId,
    ) -> AnyResult<()>;

    /// Returns the Strava activity ID associated with the given ActivityId.
    ///
    /// # Arguments
    ///
    /// * `act` - The ActivityId of the activity.
    ///
    /// # Returns
    ///
    /// The Strava activity ID associated with the given ActivityId.
    ///
    /// # Errors
    ///
    /// Returns an error if the Strava activity ID cannot be retrieved from the database.
    async fn strava_activitid_get_by_tbid(&mut self, act: i32) -> Result<i64, anyhow::Error>;

    /// Deletes the Strava activity with the given activity ID.
    ///
    /// # Arguments
    ///
    /// * `act_id` - The ID of the activity to delete.
    ///
    /// # Returns
    ///
    /// The number of rows affected by the delete operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the activity cannot be deleted.
    async fn strava_activity_delete(&mut self, act_id: i64) -> AnyResult<usize>;

    /// Returns the ActivityId associated with the given Strava activity ID.
    ///
    /// # Arguments
    ///
    /// * `act_id` - The Strava ID of the activity.
    ///
    /// # Returns
    ///
    /// The ActivityId associated with the given Strava activity ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the ActivityId cannot be retrieved from the database.
    async fn strava_activity_get_activityid(
        &mut self,
        act_id: i64,
    ) -> AnyResult<Option<ActivityId>>;

    /// Creates a new Strava gear with the given Strava ID, PartId, and user ID.
    ///
    /// # Arguments
    ///
    /// * `strava_id` - The Strava ID of the gear.
    /// * `tbid` - The PartId associated with the gear.
    /// * `user` - The user ID associated with the gear.
    ///
    /// # Errors
    ///
    /// Returns an error if the gear cannot be created.
    async fn strava_gear_new(
        &mut self,
        strava_id: String,
        tbid: PartId,
        user: UserId,
    ) -> AnyResult<()>;

    /// Deletes the Strava event with the given event ID.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the event to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be deleted.
    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> AnyResult<()>;

    /// Sets the time of the Strava event with the given event ID.
    ///
    /// # Arguments
    ///
    /// * `e_id` - The ID of the event to update.
    /// * `e_time` - The new time to set for the event.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be updated.
    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> AnyResult<()>;

    /// Stores the given Strava event.
    ///
    /// # Arguments
    ///
    /// * `e` - The event to store.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be stored.
    async fn stravaevent_store(&mut self, e: Event) -> AnyResult<()>;

    /// Returns the next Strava event for the given user.
    ///
    /// # Arguments
    ///
    /// * `user` - The user to get the next event for.
    ///
    /// # Returns
    ///
    /// The next Strava event for the given user, if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be retrieved.
    async fn strava_event_get_next_for_user(
        &mut self,
        user: &impl StravaPerson,
    ) -> AnyResult<Option<Event>>;

    /// Returns all Strava events with a start time later than the given time and associated with the given object ID and Strava ID.
    ///
    /// # Arguments
    ///
    /// * `obj_id` - The ID of the object associated with the events.
    /// * `oid` - The Strava ID of the object associated with the events.
    ///
    /// # Returns
    ///
    /// All Strava events with a start time later than the given time and associated with the given object ID and Strava ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be retrieved.
    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId)
        -> AnyResult<Vec<Event>>;

    /// Deletes the Strava events with the given event IDs.
    ///
    /// # Arguments
    ///
    /// * `values` - The IDs of the events to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be deleted.
    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> AnyResult<()>;

    /// Returns all Strava users.
    ///
    /// # Returns
    ///
    /// All Strava users.
    ///
    /// # Errors
    ///
    /// Returns an error if the users cannot be retrieved.
    async fn stravausers_get_all(&mut self) -> AnyResult<Vec<StravaUser>>;

    /// Returns the Strava user associated with the given user ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The user ID of the user.
    ///
    /// # Returns
    ///
    /// The Strava user associated with the given user ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be retrieved.
    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> AnyResult<StravaUser>;

    /// Returns all Strava users associated with the given Strava ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The Strava ID of the user.
    ///
    /// # Returns
    ///
    /// All Strava users associated with the given Strava ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the users cannot be retrieved.
    async fn stravauser_get_by_stravaid(&mut self, id: StravaId) -> AnyResult<Vec<StravaUser>>;

    /// Creates a new Strava user.
    ///
    /// # Arguments
    ///
    /// * `user` - The Strava user to create.
    ///
    /// # Returns
    ///
    /// The newly created Strava user.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be created.
    async fn stravauser_new(&mut self, user: StravaUser) -> AnyResult<StravaUser>;

    /// Updates the last activity time for a Strava user.
    ///
    /// # Arguments
    ///
    /// * `user` - The Strava user to update.
    /// * `time` - The new last activity time.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be updated.
    async fn stravauser_update_last_activity(
        &mut self,
        user: &StravaId,
        time: i64,
    ) -> AnyResult<()>;

    /// Updates the access token for a Strava user.
    ///
    /// # Arguments
    ///
    /// * `stravaid` - The Strava ID of the user.
    /// * `access` - The new access token.
    /// * `exp` - The expiration time of the access token.
    /// * `refresh` - The new refresh token, if any.
    ///
    /// # Returns
    ///
    /// The updated Strava user.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be updated.
    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> AnyResult<StravaUser>;

    /// Returns the number of Strava events for a given user.
    ///
    /// # Arguments
    ///
    /// * `user` - The Strava user to get the event count for.
    ///
    /// # Returns
    ///
    /// The number of Strava events for the given user.
    ///
    /// # Errors
    ///
    /// Returns an error if the event count cannot be retrieved.
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> AnyResult<i64>;

    /// Disables a Strava user.
    ///
    /// # Arguments
    ///
    /// * `user` - The Strava ID of the user to disable.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be disabled.
    async fn stravauser_disable(&mut self, user: &StravaId) -> AnyResult<()>;

    /// Locks a Strava ID.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The Strava ID to lock.
    ///
    /// # Returns
    ///
    /// `true` if the lock was successful, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock cannot be acquired.
    async fn stravaid_lock(&mut self, user_id: &StravaId) -> AnyResult<bool>;

    /// Unlocks a Strava ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The Strava ID to unlock.
    ///
    /// # Returns
    ///
    /// The number of rows affected by the unlock operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the unlock operation fails.
    async fn stravaid_unlock(&mut self, id: &StravaId) -> AnyResult<usize>;
}

#[async_trait]
pub trait StravaPerson: Person {
    /// Returns the Strava ID of the user.
    ///
    /// # Returns
    ///
    /// The Strava ID of the user.
    fn strava_id(&self) -> StravaId;

    fn tb_id(&self) -> UserId {
        self.get_id()
    }

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        conn: &mut impl StravaStore,
    ) -> AnyResult<T>;
}
