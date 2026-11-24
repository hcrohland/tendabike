//! The provider-agnostic event queue (spec #1023): the queue's read-side
//! rules and the failure policy, written once and generic over the event
//! type.
//!
//! Two traits: [`QueuedEvent`] (the event row's identity — its `id`, its
//! `owner`, and its dedup `key`) and [`EventQueue`] (the store ops the
//! queue's read and failure ops drive). The unified store implements
//! [`EventQueue`] once per provider's event type; the drivers implement
//! [`QueuedEvent`] for their wire shape.
//!
//! The two generic ops: [`get_event`] (the read-side rules — the rate-limit
//! gate, then the latest-per-key dedup, ADR-0005 / spec #446 §4.3) and
//! [`fail_event`] (the four-arm failure policy, spec #446 §4.6). Both were a
//! clean duplicate across the two drivers; they now live here, so a driver's
//! `event` module keeps only its wire shape plus the two trait impls.

use std::fmt::{Debug, Display};

use log::{debug, warn};

use crate::Provider;
use crate::facade::{Error, Summary, TbResult};
use crate::lane::QueueRead;

/// A queued provider event row: the identity the queue's read and failure
/// ops key on.
///
/// The [`QueuedEvent`] impl is the driver's wire shape (the `strava_events`
/// row, the `icu_events` row); the queue ops are generic over it.
pub trait QueuedEvent: Clone + Debug + Send + Sync + 'static {
    /// The provider's user id the event belongs to (the queue's partition
    /// key — the `owner_id` / `icu_athlete_id` column).
    type Owner: Clone + Debug + Send + Sync + Display + 'static;
    /// The dedup key: the object the event refers to (the Strava `object_id`,
    /// the icu `activity_id`). Only the latest event per key is interesting;
    /// the older duplicates are dropped by the read.
    type Key: Clone + Debug + Send + Sync + 'static;

    /// The row id (`None` until the row is stored).
    fn id(&self) -> Option<i32>;
    /// The owner (the provider's user id).
    fn owner(&self) -> &Self::Owner;
    /// The dedup key.
    fn key(&self) -> &Self::Key;
}

/// The provider's event queue: the store ops the kit's [`get_event`] (the
/// read-side rules) and [`fail_event`] (the failure policy) drive.
///
/// The unified store implements it once per provider's event type
/// (`EventQueue<StravaEvent>`, `EventQueue<IcuEvent>`); the driver tests
/// drive it through the data-store mock's thin forward.
///
/// The bound is `Send` (not `Sync`): every op takes `&mut self` (exclusive
/// access inside the caller's transaction), so shared concurrent access is
/// not a requirement — the unified store (`SqlxConn`, a `PgTransaction`) is
/// `Send` but not `Sync`.
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
#[async_trait::async_trait]
pub trait EventQueue<E: QueuedEvent>: Send {
    /// The provider this queue belongs to (the log messages' subject).
    fn provider_id(&self) -> Provider;

    /// The oldest queued event for `user` (the queue's partition key), if
    /// any.
    async fn get_next_for_user(&mut self, user: E::Owner) -> TbResult<Option<E>>;

    /// All queued events for `(key, owner)`, oldest first — the dedup's
    /// input (only the latest per key is interesting).
    async fn get_later(&mut self, key: E::Key, owner: E::Owner) -> TbResult<Vec<E>>;

    /// Delete the queued events whose ids are in `values` (the dedup's
    /// older-duplicate drop).
    async fn delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;

    /// Delete the queued event `event_id`.
    async fn delete_event(&mut self, event_id: Option<i32>) -> TbResult<()>;

    /// Disable `user` (the 401's policy: drop their queued events, clear
    /// their token).
    async fn disable_user(&mut self, user: E::Owner) -> TbResult<()>;

    /// The proactive rate-limit gate: the unix time the exhausted window
    /// resets, or `None` if budget remains (or no data yet).
    fn limited_until(&self) -> Option<i64>;
}

/// Read the next queued event for `owner`, with the queue's read-side rules
/// (ADR-0005, executable spec #446 §4.3):
///
/// - the proactive rate-limit gate is checked first: budget exhausted →
///   [`QueueRead::Limited`] (the event stays queued);
/// - only the latest event per key is interesting: older events for the same
///   key are deleted and the latest is returned.
///
/// The read runs inside the caller's transaction (the deletions above commit
/// or roll back with it).
pub(crate) async fn get_event<E, Q>(owner: E::Owner, queue: &mut Q) -> TbResult<QueueRead<E>>
where
    E: QueuedEvent,
    Q: EventQueue<E>,
{
    // Proactive rate-limit gate: if the budget is exhausted, report the
    // window boundary without dequeuing (the event stays queued).
    if let Some(until) = queue.limited_until() {
        return Ok(QueueRead::Limited { until });
    }

    let event = match queue.get_next_for_user(owner).await? {
        Some(event) => event,
        None => return Ok(QueueRead::Empty),
    };

    // Prevent unneeded provider calls: only the latest event for a key is
    // interesting (a burst of webhooks on one object collapses to its newest
    // row; the older duplicates are dropped).
    let mut list = queue
        .get_later(event.key().clone(), event.owner().clone())
        .await?;
    let res = list.pop();

    if !list.is_empty() {
        debug!("Dropping {list:#?}");
        let values = list.into_iter().map(|l| l.id()).collect::<Vec<_>>();
        queue.delete_batch(values).await?;
    }

    Ok(match res {
        Some(event) => QueueRead::Event(event),
        None => QueueRead::Empty,
    })
}

/// Applies the kit's failure decision ([`crate::failure`], spec #446 §4.6) to
/// the queue's tables: the disable and the delete commit with the caller's
/// transaction.
///
/// Every arm returns `Ok` except the DB failure (`Propagate`): the queue-side
/// effects (the disable, the delete) are committed by the caller's
/// transaction, and the loop continues. The event is deleted in exactly one
/// place on `DropEvent` (here) — no second delete on the caller's fall-through.
pub(crate) async fn fail_event<E, Q>(event: &E, err: Error, queue: &mut Q) -> TbResult<Summary>
where
    E: QueuedEvent,
    Q: EventQueue<E>,
{
    let provider = queue.provider_id().wire_value();
    match crate::failure::classify(&err) {
        crate::failure::FailureDecision::KeepQueued => {
            // The rate-limit state is already updated from the 429 response
            // headers; the event stays queued and the next `get_event` probe
            // will report `Limited`.
            warn!("{provider} rate limited ({err}); event stays queued until the window resets");
            Ok(Summary::default())
        }
        crate::failure::FailureDecision::DisableUser => {
            warn!(
                "{provider} auth is dead ({err}); disabling user {}",
                event.owner()
            );
            queue.disable_user(event.owner().clone()).await?;
            Ok(Summary::default())
        }
        crate::failure::FailureDecision::Propagate => Err(err),
        crate::failure::FailureDecision::DropEvent => {
            warn!("dropping poison event (owner: {}): {err:#}", event.owner());
            queue.delete_event(event.id()).await?;
            Ok(Summary::default())
        }
    }
}

#[cfg(test)]
mod queue_tests;
