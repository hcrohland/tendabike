/*
This file defines the StravaStore trait, which extends the tb_domain::traits::Store trait
and provides additional methods for interacting with Strava data.
*/

use serde::de::DeserializeOwned;

use crate::{StravaId, StravaUser, event::Event};
use tb_domain::Domain;

#[async_trait::async_trait]
pub trait StravaStore<D: Domain>: Send {
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
    async fn stravaid_get_user_id(&mut self, who: i32) -> Result<i32, D::Error>;

    /// Deletes the Strava event with the given event ID.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the event to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be deleted.
    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> Result<(), D::Error>;

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
    async fn strava_event_set_time(
        &mut self,
        e_id: Option<i32>,
        e_time: i64,
    ) -> Result<(), D::Error>;

    /// Stores the given Strava event.
    ///
    /// # Arguments
    ///
    /// * `e` - The event to store.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be stored.
    async fn stravaevent_store(&mut self, e: Event) -> Result<(), D::Error>;

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
        user: StravaId,
    ) -> Result<Option<Event>, D::Error>;

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
    async fn strava_event_get_later(
        &mut self,
        obj_id: i64,
        oid: StravaId,
    ) -> Result<Vec<Event>, D::Error>;

    /// Deletes the Strava events with the given event IDs.
    ///
    /// # Arguments
    ///
    /// * `values` - The IDs of the events to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be deleted.
    async fn strava_events_delete_batch(
        &mut self,
        values: Vec<Option<i32>>,
    ) -> Result<(), D::Error>;

    /// Returns all Strava users.
    ///
    /// # Returns
    ///
    /// All Strava users.
    ///
    /// # Errors
    ///
    /// Returns an error if the users cannot be retrieved.
    async fn stravausers_get_all(&mut self) -> Result<Vec<StravaUser>, D::Error>;

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
    async fn stravauser_get_by_tbid(&mut self, id: D::UserId) -> Result<StravaUser, D::Error>;

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
    async fn stravauser_get_by_stravaid(
        &mut self,
        id: &StravaId,
    ) -> Result<Option<StravaUser>, D::Error>;

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
    async fn stravauser_new(&mut self, user: StravaUser) -> Result<StravaUser, D::Error>;

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
    ) -> Result<StravaUser, D::Error>;

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
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> Result<i64, D::Error>;

    /// Deletes a[[ Strava events for a given user.
    ///
    /// # Arguments
    ///
    /// * `user` - The Strava user to get the event count for.
    ///
    /// # Returns
    ///
    /// The number of Strava events deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if the event count cannot be retrieved.
    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> Result<usize, D::Error>;

    async fn stravauser_delete(&mut self, user: D::UserId) -> Result<usize, D::Error>;
}

#[async_trait::async_trait]
pub trait StravaSession<D: Domain> {
    /// Returns the Strava ID of the user.
    ///
    /// # Returns
    ///
    /// The Strava ID of the user.
    fn strava_id(&self) -> StravaId;

    fn tb_id(&self) -> D::UserId;

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        store: &mut impl StravaStore<D>,
    ) -> Result<T, D::Error>;

    async fn deauthorize(&mut self, store: &mut impl StravaStore<D>) -> Result<(), D::Error>;
}
