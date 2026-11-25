/*
This file defines the Strava data-store traits.

`StravaDataStore` is the local trait: it carries the Strava-specific methods
(the `strava_users` and `strava_events` tables) and no domain supertrait, so a
consumer that only drives Strava work can bound on it directly.

`StravaStore` is the method-free umbrella over the domain `Store` +
`StravaDataStore`, naming a store that can do both domain and Strava work —
the same "local traits plus an umbrella" shape as `Store` itself.
*/

use super::{StravaId, StravaUser, event::Event};
use crate::Provider;
use crate::ProviderSpec;
use crate::facade::{Store, TbResult, UserId};

/// The Strava-side data store: the `strava_users` and `strava_events` tables.
///
/// This is the local trait — it carries no domain supertrait, so a consumer
/// that only drives Strava work can bound on it directly instead of the full
/// [`StravaStore`] umbrella.
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
#[async_trait::async_trait]
pub trait StravaDataStore: Send + Sync {
    /// Returns all Strava users.
    ///
    /// # Errors
    ///
    /// Returns an error if the users cannot be retrieved.
    async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>>;

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
    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser>;

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
    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>>;

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
    async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser>;

    /// Updates the access token for a Strava user.
    ///
    /// # Arguments
    ///
    /// * `stravaid` - The Strava ID of the user.
    /// * `refresh` - The new refresh token, if any.
    ///
    /// # Returns
    ///
    /// The updated Strava user.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be updated.
    async fn stravauser_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<String>,
    ) -> TbResult<StravaUser>;

    /// Deletes the Strava user with the given TendaBike user ID.
    ///
    /// # Arguments
    ///
    /// * `user` - The TendaBike user ID of the user.
    ///
    /// # Returns
    ///
    /// The number of Strava users deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if the user cannot be deleted.
    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize>;

    /// Stores the given Strava event.
    ///
    /// # Arguments
    ///
    /// * `e` - The event to store.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be stored.
    async fn strava_event_create(&mut self, e: Event) -> TbResult<()>;

    /// Deletes the Strava event with the given event ID.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the event to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be deleted.
    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()>;

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
    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()>;

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
    async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>>;

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
    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>>;

    /// Deletes the Strava events with the given event IDs.
    ///
    /// # Arguments
    ///
    /// * `values` - The IDs of the events to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be deleted.
    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;

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
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64>;

    /// Deletes all Strava events for a given user.
    ///
    /// # Arguments
    ///
    /// * `user` - The Strava user whose events should be deleted.
    ///
    /// # Returns
    ///
    /// The number of Strava events deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be deleted.
    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize>;
}

/// A method-free umbrella naming a complete Strava store: one that implements
/// the domain [`Store`] *and* the local [`StravaDataStore`]. It
/// carries no methods of its own — the same shape as `Store`,
/// which is the umbrella over the domain's per-entity sub-traits.
pub trait StravaStore: Store + StravaDataStore {}

/// The Strava provider marker (spec #1029): the type carrier for Strava's wire
/// shapes — the [`StravaUser`] row, the [`Event`] row, the [`StravaId`] owner,
/// the `i64` dedup key (the `object_id`).
pub struct StravaSpec;

impl ProviderSpec for StravaSpec {
    const PROVIDER: Provider = Provider::Strava;
    type User = StravaUser;
    type Event = Event;
    type Owner = StravaId;
    type Key = i64;
    fn make_sync(owner: StravaId, time: i64) -> Event {
        Event {
            id: None,
            object_type: super::event::ObjectType::Sync,
            object_id: 0,
            owner_id: owner,
            event_time: time,
            ..Default::default()
        }
    }
    fn event_id(e: &Event) -> Option<i32> {
        e.id
    }
    fn event_owner(e: &Event) -> &StravaId {
        &e.owner_id
    }
    fn event_key(e: &Event) -> &i64 {
        &e.object_id
    }
    fn user_tbid(user: &StravaUser) -> &UserId {
        &user.tendabike_id
    }
    fn user_owner(user: &StravaUser) -> &StravaId {
        &user.id
    }
}

/// The provider-agnostic store over the Strava data-store mock (spec #1029,
/// ticket #1031): a thin forward so the driver's `process` (generic over the
/// [`Store`](crate::store::Store)) and the kit's queue ops run on the mock in
/// the DB-free suites. The ops map one-to-one onto the `StravaDataStore`
/// expectations; `user_disable` and `limited_until` drive the driver's own
/// policy (the `StravaId` disable, the process-wide limiter).
#[cfg(any(test, feature = "test-support"))]
#[async_trait::async_trait]
impl crate::store::Store<StravaSpec> for MockStravaDataStore {
    async fn user_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        self.stravauser_get_all().await
    }
    async fn user_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        self.stravauser_get_by_tbid(id).await
    }
    async fn user_get_by_provider_id(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        self.stravauser_get_by_stravaid(id).await
    }
    async fn user_create(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        self.stravauser_create(user).await
    }
    async fn user_update_token(
        &mut self,
        owner: StravaId,
        token: Option<String>,
    ) -> TbResult<StravaUser> {
        self.stravauser_update_token(owner, token).await
    }
    async fn user_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.stravauser_delete(user).await
    }
    async fn user_disable(&mut self, owner: StravaId) -> TbResult<()> {
        let _ = self.strava_events_delete_for_user(&owner).await?;
        self.stravauser_update_token(owner, None).await?;
        Ok(())
    }
    async fn user_event_count(&mut self, owner: &StravaId) -> TbResult<i64> {
        self.strava_events_get_count_for_user(owner).await
    }
    async fn event_get_next(&mut self, owner: StravaId) -> TbResult<Option<Event>> {
        self.strava_event_get_next_for_user(owner).await
    }
    async fn event_get_later(&mut self, key: i64, owner: StravaId) -> TbResult<Vec<Event>> {
        self.strava_event_get_later(key, owner).await
    }
    async fn event_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.strava_events_delete_batch(values).await
    }
    async fn event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.strava_event_delete(event_id).await
    }
    async fn event_set_time(&mut self, event_id: Option<i32>, time: i64) -> TbResult<()> {
        self.strava_event_set_time(event_id, time).await
    }
    async fn event_delete_sync(&mut self, owner: &StravaId) -> TbResult<usize> {
        // The Strava syncs are the owner's `object_id` 0 events.
        let syncs = self.strava_event_get_later(0, *owner).await?;
        let ids = syncs.iter().map(|e| e.id).collect();
        self.strava_events_delete_batch(ids).await?;
        Ok(syncs.len())
    }
    async fn event_create(&mut self, e: Event) -> TbResult<()> {
        self.strava_event_create(e).await
    }
    fn limited_until(&self) -> Option<i64> {
        super::ratelimit::limited_until()
    }
}

#[cfg(test)]
mod tests {
    use super::super::event::ObjectType;
    use super::*;

    /// The marker's `PROVIDER` constant is the enum value (the marker → enum
    /// direction).
    #[test]
    fn strava_spec_provider_constant() {
        assert_eq!(StravaSpec::PROVIDER, Provider::Strava);
    }

    /// The marker's `make_sync` builds the durable sync row: a `Sync` for the
    /// owner at `time`, `object_id` 0, no row id yet.
    #[test]
    fn strava_spec_make_sync_builds_sync_row() {
        let owner = StravaId::from(42);
        let event = StravaSpec::make_sync(owner, 1_700_000_000);
        assert_eq!(event.object_type, ObjectType::Sync);
        assert_eq!(event.object_id, 0);
        assert_eq!(event.owner_id, owner);
        assert_eq!(event.event_time, 1_700_000_000);
        assert_eq!(event.id, None);
    }

    /// The marker's event-identity accessors read the row's id, owner, and key.
    #[test]
    fn strava_spec_event_identity_accessors() {
        let event = StravaSpec::make_sync(StravaId::from(42), 1_700_000_000);
        assert_eq!(StravaSpec::event_id(&event), None);
        assert_eq!(*StravaSpec::event_owner(&event), StravaId::from(42));
        assert_eq!(*StravaSpec::event_key(&event), 0);
    }
}
