//! The provider-agnostic store (spec #1029, ticket #1030): the single uniform
//! store interface, generic over the [`ProviderSpec`](crate::ProviderSpec)
//! marker.
//!
//! [`Store`] carries the user-row operations (get-all, get-by-tbid,
//! get-by-provider-id, create, update-token, delete, disable, event-count) and
//! the event-row operations (get-next, get-later, delete-batch, delete, create,
//! the rate-limit gate). The database crate implements it once per provider, as
//! thin forwards to the existing per-provider SQL operations. It is
//! app-internal — not part of the driver author's surface.
//!
//! Under `test-support`, `mockall`'s `#[automock]` generates a single
//! [`MockStore`] double for the DB-free suites.

use crate::ProviderSpec;
use crate::facade::{TbResult, UserId};

/// The provider-agnostic store (spec #1029): the single uniform store
/// interface, generic over the [`ProviderSpec`](crate::ProviderSpec) marker.
///
/// Carries the user-row operations and the event-row operations (including the
/// rate-limit gate). The database crate implements it once per provider, as
/// thin forwards to the existing per-provider SQL operations. It is
/// app-internal — not part of the driver author's surface.
///
/// The bound is `Send + Sync`: every op takes `&mut self` (exclusive access
/// inside the caller's transaction) except the rate-limit gate, so the store is
/// shared across the executor's lanes.
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
#[async_trait::async_trait]
pub trait Store<P: ProviderSpec>: Send + Sync {
    // ── user-row operations ────────────────────────────────────────────────
    /// All of the provider's users.
    async fn user_get_all(&mut self) -> TbResult<Vec<P::User>>;

    /// The provider's user bound to `id` (the TendaBike user id).
    async fn user_get_by_tbid(&mut self, id: UserId) -> TbResult<P::User>;

    /// The provider's user bound to `id` (the provider id), if any.
    async fn user_get_by_provider_id(&mut self, id: &P::Owner) -> TbResult<Option<P::User>>;

    /// Insert a new provider user and return the stored row.
    async fn user_create(&mut self, user: P::User) -> TbResult<P::User>;

    /// Set the token on the provider user `owner`; `None` clears it.
    async fn user_update_token(
        &mut self,
        owner: P::Owner,
        token: Option<String>,
    ) -> TbResult<P::User>;

    /// Delete the provider user bound to `user` (the TendaBike user id);
    /// returns the number of rows deleted.
    async fn user_delete(&mut self, user: UserId) -> TbResult<usize>;

    /// Disable the provider user `owner`: drop their queued events and clear
    /// their token (the 401's policy).
    async fn user_disable(&mut self, owner: P::Owner) -> TbResult<()>;

    /// The number of queued events for `owner`.
    async fn user_event_count(&mut self, owner: &P::Owner) -> TbResult<i64>;

    // ── event-row operations ───────────────────────────────────────────────
    /// The oldest queued event for `owner` (the queue's partition key), if any.
    async fn event_get_next(&mut self, owner: P::Owner) -> TbResult<Option<P::Event>>;

    /// All queued events for `(key, owner)`, oldest first — the dedup's input
    /// (only the latest per key is interesting).
    async fn event_get_later(&mut self, key: P::Key, owner: P::Owner) -> TbResult<Vec<P::Event>>;

    /// Delete the queued events whose ids are in `values` (the dedup's
    /// older-duplicate drop).
    async fn event_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;

    /// Delete the queued event `event_id`.
    async fn event_delete(&mut self, event_id: Option<i32>) -> TbResult<()>;

    /// Set the time of the queued event `event_id` (the sync cursor's
    /// advance — the durable backfill position).
    async fn event_set_time(&mut self, event_id: Option<i32>, time: i64) -> TbResult<()>;

    /// Delete the queued sync events for `owner` (the durable backfill
    /// cursor's REPLACE enqueue: at most one sync per user); returns the
    /// number of rows deleted.
    async fn event_delete_sync(&mut self, owner: &P::Owner) -> TbResult<usize>;

    /// Queue a provider event (a webhook/sync row).
    async fn event_create(&mut self, e: P::Event) -> TbResult<()>;

    /// The proactive rate-limit gate: the unix time the exhausted window
    /// resets, or `None` if budget remains (or no data yet).
    fn limited_until(&self) -> Option<i64>;
}

/// The "all provider stores" bound (ADR-0014): the single home for the
/// composition of every provider's [`Store`] marker —
/// `Store<StravaSpec> + Store<IcuSpec> + Send + Sync`. A third provider adds
/// its `Store` marker here (in the provider layer only); the direct-op
/// dispatch fns, [`provider_of`](crate::provider_of), and the executor's
/// `ProviderService` impl all bound on this one stable trait, so the executor
/// crate is untouched by a new provider.
///
/// A supertrait: any type carrying the per-provider [`Store`] markers and the
/// `Send + Sync` liveness implements it with an empty body (the composite
/// test doubles already do — the executor's `ProviderService` impl is testable
/// with such a mock store).
#[cfg(all(feature = "strava", feature = "icu"))]
pub trait ProviderStore:
    Store<crate::strava::StravaSpec> + Store<crate::icu::IcuSpec> + Send + Sync
{
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::strava::StravaSpec;
    use crate::test_support::MockStore;

    /// The `#[automock]` witness is the seam a DB-free provider test mocks: set
    /// an expectation and the canned outcome is served without touching any
    /// store. The single [`MockStore`] double stands in for the per-provider
    /// data-store mocks (spec #1029, ticket #1030).
    #[tokio::test]
    async fn mock_store_serves_canned_outcome() {
        let mut mock = MockStore::<StravaSpec>::new();
        mock.expect_user_get_all().returning(|| Ok(Vec::new()));
        let users = mock.user_get_all().await.unwrap();
        assert!(users.is_empty());
    }
}
