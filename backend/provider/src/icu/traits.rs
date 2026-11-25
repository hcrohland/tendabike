/*
This file defines the intervals.icu data-store traits.

`IcuDataStore` is the local trait: it carries the icu-specific methods (the
`icu_users` and `icu_events` tables) and no domain supertrait, so a consumer
that only drives icu work can bound on it directly.

`IcuStore` is the method-free umbrella over the domain `Store` +
`IcuDataStore`, naming a store that can do both domain and icu work — the
same "local traits plus an umbrella" shape as `Store` itself.
*/

use super::{IcuEvent, IcuId, IcuUser};
use crate::Provider;
use crate::ProviderSpec;
use crate::facade::{Store, TbResult, UserId};

/// The icu-side data store: the `icu_users` and `icu_events` tables.
///
/// This is the local trait — it carries no domain supertrait, so a consumer
/// that only drives icu work can bound on it directly instead of the full
/// [`IcuStore`] umbrella. Every method returns a [`TbResult`] (a `DatabaseFailure`
/// when the backing store is unreachable).
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
#[async_trait::async_trait]
pub trait IcuDataStore: Send + Sync {
    // ── icu_users ────────────────────────────────────────────────────────
    /// All icu users.
    async fn icuser_get_all(&mut self) -> TbResult<Vec<IcuUser>>;

    /// The icu user bound to `id` (the TendaBike user id).
    async fn icuser_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser>;

    /// The icu user bound to `id` (the icu athlete id), if any.
    async fn icuser_get_by_icuid(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>>;

    /// Insert a new icu user and return the stored row.
    async fn icuser_create(&mut self, user: IcuUser) -> TbResult<IcuUser>;

    /// Set the (durable) access token on the icu user `icuid`; `None` clears it.
    async fn icuser_update_token(
        &mut self,
        icuid: IcuId,
        access_token: Option<String>,
    ) -> TbResult<IcuUser>;

    /// Delete the icu user bound to `user` (the TendaBike user id); returns the
    /// number of rows deleted.
    async fn icuser_delete(&mut self, user: UserId) -> TbResult<usize>;

    // ── icu_events ───────────────────────────────────────────────────────
    /// Queue an icu event (a webhook/sync row).
    async fn icu_event_create(&mut self, e: IcuEvent) -> TbResult<()>;

    /// Delete the queued icu event `event_id`.
    async fn icu_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()>;

    /// Move the queued icu event `e_id` to time `e_time` (the backfill cursor).
    async fn icu_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()>;

    /// The oldest queued icu event for `user` (the icu athlete id), if any.
    async fn icu_event_get_next_for_user(&mut self, user: IcuId) -> TbResult<Option<IcuEvent>>;

    /// All queued icu events for the given activity (the
    /// `(activity_id, icu_athlete_id)` dedup key), oldest first (the queue's
    /// read-side dedup: only the latest per activity is interesting, spec
    /// #996).
    async fn icu_event_get_later(
        &mut self,
        activity_id: &str,
        icu_athlete_id: &IcuId,
    ) -> TbResult<Vec<IcuEvent>>;

    /// Delete the queued icu events whose ids are in `values` (the dedup's
    /// older-duplicate drop, spec #996).
    async fn icu_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;

    /// The number of queued icu events for `user` (the icu athlete id).
    async fn icu_events_get_count_for_user(&mut self, user: &IcuId) -> TbResult<i64>;

    /// The newest queued icu event's time for `user` (the icu athlete id) —
    /// the onboarding backfill's resume cursor — or `None` when the user has
    /// no queued events.
    async fn icu_events_get_latest_time(&mut self, user: &IcuId) -> TbResult<Option<i64>>;

    /// Delete every queued icu event for `user` (the icu athlete id); returns
    /// the number of rows deleted.
    async fn icu_events_delete_for_user(&mut self, user: &IcuId) -> TbResult<usize>;

    /// Delete the user's queued `SYNC` event (the REPLACE enqueue: at most
    /// one `SYNC` per user); returns the number of rows deleted.
    async fn icu_events_delete_sync_for_user(&mut self, user: &IcuId) -> TbResult<usize>;
}

/// A method-free umbrella naming a complete icu store: one that implements
/// the domain [`Store`] *and* the local [`IcuDataStore`]. It
/// carries no methods of its own — the same shape as `Store`,
/// which is the umbrella over the domain's per-entity sub-traits.
pub trait IcuStore: Store + IcuDataStore {}

/// The icu provider marker (spec #1029): the type carrier for icu's wire
/// shapes — the [`IcuUser`] row, the [`IcuEvent`] row, the [`IcuId`] owner,
/// the `String` dedup key (the `activity_id`).
pub struct IcuSpec;

impl ProviderSpec for IcuSpec {
    const PROVIDER: Provider = Provider::Icu;
    type User = IcuUser;
    type Event = IcuEvent;
    type Owner = IcuId;
    type Key = String;
    fn make_sync(owner: IcuId, time: i64) -> IcuEvent {
        IcuEvent::new_sync(owner, time)
    }
    fn event_id(e: &IcuEvent) -> Option<i32> {
        e.id
    }
    fn event_owner(e: &IcuEvent) -> &IcuId {
        &e.icu_athlete_id
    }
    fn event_key(e: &IcuEvent) -> &String {
        &e.activity_id
    }
    fn user_tbid(user: &IcuUser) -> &UserId {
        &user.tendabike_id
    }
    fn user_owner(user: &IcuUser) -> &IcuId {
        &user.id
    }
}

/// The provider-agnostic store over the icu data-store mock (spec #1029,
/// ticket #1031): a thin forward so the driver's `process` (generic over the
/// [`Store`](crate::store::Store)) and the kit's queue ops run on the mock in
/// the DB-free suites. The ops map one-to-one onto the `IcuDataStore`
/// expectations; `user_disable` and `limited_until` drive the driver's own
/// policy (the `IcuId` disable, the process-wide limiter).
#[cfg(any(test, feature = "test-support"))]
#[async_trait::async_trait]
impl crate::store::Store<IcuSpec> for MockIcuDataStore {
    async fn user_get_all(&mut self) -> TbResult<Vec<IcuUser>> {
        self.icuser_get_all().await
    }
    async fn user_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser> {
        self.icuser_get_by_tbid(id).await
    }
    async fn user_get_by_provider_id(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>> {
        self.icuser_get_by_icuid(id).await
    }
    async fn user_create(&mut self, user: IcuUser) -> TbResult<IcuUser> {
        self.icuser_create(user).await
    }
    async fn user_update_token(
        &mut self,
        owner: IcuId,
        token: Option<String>,
    ) -> TbResult<IcuUser> {
        self.icuser_update_token(owner, token).await
    }
    async fn user_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.icuser_delete(user).await
    }
    async fn user_disable(&mut self, owner: IcuId) -> TbResult<()> {
        let _ = self.icu_events_delete_for_user(&owner).await?;
        self.icuser_update_token(owner, None).await?;
        Ok(())
    }
    async fn user_event_count(&mut self, owner: &IcuId) -> TbResult<i64> {
        self.icu_events_get_count_for_user(owner).await
    }
    async fn event_get_next(&mut self, owner: IcuId) -> TbResult<Option<IcuEvent>> {
        self.icu_event_get_next_for_user(owner).await
    }
    async fn event_get_later(&mut self, key: String, owner: IcuId) -> TbResult<Vec<IcuEvent>> {
        self.icu_event_get_later(&key, &owner).await
    }
    async fn event_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.icu_events_delete_batch(values).await
    }
    async fn event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.icu_event_delete(event_id).await
    }
    async fn event_set_time(&mut self, event_id: Option<i32>, time: i64) -> TbResult<()> {
        self.icu_event_set_time(event_id, time).await
    }
    async fn event_delete_sync(&mut self, owner: &IcuId) -> TbResult<usize> {
        self.icu_events_delete_sync_for_user(owner).await
    }
    async fn event_create(&mut self, e: IcuEvent) -> TbResult<()> {
        self.icu_event_create(e).await
    }
    fn limited_until(&self) -> Option<i64> {
        super::ratelimit::limited_until()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icu::IcuObjectType;

    /// The marker's `PROVIDER` constant is the enum value (the marker → enum
    /// direction).
    #[test]
    fn icu_spec_provider_constant() {
        assert_eq!(IcuSpec::PROVIDER, Provider::Icu);
    }

    /// The marker's `make_sync` builds the durable sync row: a `Sync` for the
    /// owner with the `oldest` bound, the `"sync"` sentinel key, the `0`
    /// `newest` sentinel, no row id yet.
    #[test]
    fn icu_spec_make_sync_builds_sync_row() {
        let owner = IcuId::from("ath-42");
        let event = IcuSpec::make_sync(owner.clone(), 1_700_000_000);
        assert_eq!(event.object_type, IcuObjectType::Sync);
        assert_eq!(event.icu_athlete_id, owner);
        assert_eq!(event.activity_id, "sync");
        assert_eq!(event.event_time, 0);
        assert_eq!(event.oldest, Some(1_700_000_000));
        assert_eq!(event.id, None);
    }

    /// The marker's event-identity accessors read the row's id, owner, and key.
    #[test]
    fn icu_spec_event_identity_accessors() {
        let event = IcuSpec::make_sync(IcuId::from("ath-42"), 1_700_000_000);
        assert_eq!(IcuSpec::event_id(&event), None);
        assert_eq!(*IcuSpec::event_owner(&event), IcuId::from("ath-42"));
        assert_eq!(*IcuSpec::event_key(&event), "sync");
    }
}
