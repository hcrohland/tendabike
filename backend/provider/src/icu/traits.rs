/*
This file defines the intervals.icu data-store traits.

`IcuDataStore` is the local trait: it carries the icu-specific methods (the
`icu_users` and `icu_events` tables) and no domain supertrait, so a consumer
that only drives icu work can bound on it directly.

`IcuStore` is the method-free umbrella over `tb_domain::Store` +
`IcuDataStore`, naming a store that can do both domain and icu work — the
same "local traits plus an umbrella" shape as `tb_domain::Store` itself.
*/

use super::{IcuEvent, IcuId, IcuUser};
use tb_domain::{TbResult, UserId};

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

    /// The number of queued icu events for `user` (the icu athlete id).
    async fn icu_events_get_count_for_user(&mut self, user: &IcuId) -> TbResult<i64>;

    /// The newest queued icu event's time for `user` (the icu athlete id) —
    /// the onboarding backfill's resume cursor — or `None` when the user has
    /// no queued events.
    async fn icu_events_get_latest_time(&mut self, user: &IcuId) -> TbResult<Option<i64>>;

    /// Delete every queued icu event for `user` (the icu athlete id); returns
    /// the number of rows deleted.
    async fn icu_events_delete_for_user(&mut self, user: &IcuId) -> TbResult<usize>;
}

/// A method-free umbrella naming a complete icu store: one that implements
/// the domain [`tb_domain::Store`] *and* the local [`IcuDataStore`]. It
/// carries no methods of its own — the same shape as `tb_domain::Store`,
/// which is the umbrella over the domain's per-entity sub-traits.
pub trait IcuStore: tb_domain::Store + IcuDataStore {}
