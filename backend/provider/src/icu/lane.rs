//! The icu provider lane: the [`ProviderLane`] impl for the icu driver.
//!
//! Lives in the icu module because the lane bridges the loop's
//! [`ProviderLane`] seam to the module's `get_event`/`process` ops — the code
//! that owns those ops owns the lane. `probe` delegates to
//! [`event::get_event`] and `process` to [`event::process`]. The lane is
//! generic over the store type `S` (the provider's full store — the
//! transaction's connection): `type Store = S`, so the loop's transaction
//! connection ties to `S` and `probe`/`process` operate on it.

use std::marker::PhantomData;

use crate::{DomainFacade, Probe, ProviderLane};
use tb_domain::{PartId, Summary, TbResult, UserId, UserStat};

use super::event::{QueueRead, get_event, process};
use super::{IcuEvent, IcuStore};

/// The icu provider lane: the [`ProviderLane`] impl for the icu driver.
///
/// Generic over the store type `S` (the provider's full store — the
/// transaction's connection): `type Store = S`. `S` is the connection the
/// loop's transaction source opens.
///
/// The marker-field impls (`Clone`/`Copy`/`Debug`/`Default`) are written by
/// hand rather than derived: the lane holds no `S` data (only a
/// `PhantomData<S>` marker), so it is `Clone`/`Copy`/`Debug`/`Default` for
/// any `S` — including a database connection, which is neither.
pub struct IcuLane<S: IcuStore> {
    _store: PhantomData<S>,
}

impl<S: IcuStore> Clone for IcuLane<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: IcuStore> Copy for IcuLane<S> {}

impl<S: IcuStore> std::fmt::Debug for IcuLane<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IcuLane").finish()
    }
}

impl<S: IcuStore> Default for IcuLane<S> {
    fn default() -> Self {
        Self {
            _store: PhantomData,
        }
    }
}

// `IcuLane<S>` holds no `S` data (only a `PhantomData<S>` marker), so it is
// safe to share across threads even when `S` is not `Sync`. The auto-derived
// `Sync` would require `S: Sync`; this sound `unsafe impl` provides the
// unconditional `Sync` the `ProviderLane: Send + Sync` bound needs.
unsafe impl<S: IcuStore> Sync for IcuLane<S> {}

impl<S: IcuStore> IcuLane<S> {
    /// The lane for a store of type `S`.
    pub fn new() -> Self {
        Self {
            _store: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<S> ProviderLane for IcuLane<S>
where
    S: IcuStore + Send + 'static,
{
    type Event = IcuEvent;
    type Store = S;

    /// Probe the icu event queue (one transaction's housekeeping): the
    /// rate-limit gate, then the next queued event. Delegates to
    /// [`event::get_event`].
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<Self::Event> {
        match get_event(user_id, store).await {
            Ok(QueueRead::Event(event)) => Probe::Event(event),
            Ok(QueueRead::Limited { until }) => Probe::Limited(until),
            Ok(QueueRead::Empty) => Probe::Empty,
            Err(_) => Probe::Error,
        }
    }

    /// Process the user's queued icu events (the work transaction): drain the
    /// queue, importing each activity. Delegates to [`event::process`].
    /// Returns the batch's [`Summary`] on success, or `None` on a DB failure.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        process(user_id, &crate::DomainFacadeImpl, store).await.ok()
    }

    /// Queue a sync event (the onboarding cutover's provider side). The icu
    /// backfill (the date-window walk) lands in a later ticket (spec #764);
    /// until then this is a no-op so the onboarding cutover commits cleanly.
    async fn insert_sync(
        &self,
        _user_id: UserId,
        _time: i64,
        _auto: bool,
        _store: &mut S,
    ) -> TbResult<()> {
        Ok(())
    }

    /// Import the user's icu gear (the hydration write's provider side). The
    /// icu gear import (`GET /athlete/{id}/gear`) lands in a later ticket
    /// (spec #764); until then this is a no-op.
    async fn update_gear(&self, _user_id: UserId, _store: &mut S) -> TbResult<Vec<PartId>> {
        Ok(Vec::new())
    }

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side): the domain [`UserStat`] — the stat plus the provider's
    /// queued-event count and disabled flag.
    async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>> {
        all_stats(store).await
    }
}

/// The all-user icu stat read (spec #764): every icu user's provider-neutral
/// [`UserStat`] — the domain stat plus this provider's queued-event count and
/// disabled flag.
async fn all_stats<S: IcuStore + Sync + 'static>(store: &mut S) -> TbResult<Vec<UserStat>> {
    let users = store.icuser_get_all().await?;
    let mut res = Vec::new();
    for u in users {
        let events = store.icu_events_get_count_for_user(&u.id).await?;
        res.push(UserStat {
            stat: crate::DomainFacadeImpl
                .get_stat(&u.tendabike_id, store)
                .await?,
            events,
            disabled: u.disabled(),
        });
    }
    Ok(res)
}
