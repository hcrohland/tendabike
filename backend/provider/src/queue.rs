//! The provider-agnostic event-queue kernel (spec #1029, ticket #1031): the
//! queue's read-side rules and the failure policy, written once and generic
//! over the [`ProviderSpec`](crate::ProviderSpec) marker.
//!
//! The event identity (id, owner, key) is carried by the marker's type-level
//! accessors; the queue operations (get-next, get-later, delete-batch,
//! delete, disable, the rate-limit gate) are driven through the single
//! [`Store`](crate::store::Store) interface (spec #1029, ticket #1031).
//!
//! The two generic ops: [`get_event`] (the read-side rules — the rate-limit
//! gate, then the latest-per-key dedup, ADR-0005 / spec #446 §4.3) and
//! [`fail_event`] (the four-arm failure policy, spec #446 §4.6).

use log::{debug, warn};

use crate::ProviderSpec;
use crate::facade::{Error, Summary, TbResult};
use crate::lane::QueueRead;
use crate::store::Store;

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
pub(crate) async fn get_event<P, S>(owner: P::Owner, store: &mut S) -> TbResult<QueueRead<P::Event>>
where
    P: ProviderSpec,
    S: Store<P>,
{
    // Proactive rate-limit gate: if the budget is exhausted, report the
    // window boundary without dequeuing (the event stays queued).
    if let Some(until) = store.limited_until() {
        return Ok(QueueRead::Limited { until });
    }

    let event = match store.event_get_next(owner).await? {
        Some(event) => event,
        None => return Ok(QueueRead::Empty),
    };

    // Prevent unneeded provider calls: only the latest event for a key is
    // interesting (a burst of webhooks on one object collapses to its newest
    // row; the older duplicates are dropped).
    let mut list = store
        .event_get_later(P::event_key(&event).clone(), P::event_owner(&event).clone())
        .await?;
    let res = list.pop();

    if !list.is_empty() {
        debug!("Dropping {list:#?}");
        let values = list
            .into_iter()
            .map(|l| P::event_id(&l))
            .collect::<Vec<_>>();
        store.event_delete_batch(values).await?;
    }

    Ok(match res {
        Some(event) => QueueRead::Event(event),
        None => QueueRead::Empty,
    })
}

/// Applies the kit's failure decision ([`crate::failure`], spec #446 §4.6) to
/// the store's tables: the disable and the delete commit with the caller's
/// transaction.
///
/// Every arm returns `Ok` except the DB failure (`Propagate`): the store-side
/// effects (the disable, the delete) are committed by the caller's
/// transaction, and the loop continues. The event is deleted in exactly one
/// place on `DropEvent` (here) — no second delete on the caller's fall-through.
///
/// The user-disable arm is a store user-lifecycle op
/// ([`Store::user_disable`](crate::store::Store::user_disable)) (spec #1029,
/// ticket #1031).
pub(crate) async fn fail_event<P, S>(
    event: &P::Event,
    err: Error,
    store: &mut S,
) -> TbResult<Summary>
where
    P: ProviderSpec,
    S: Store<P>,
{
    let provider = P::PROVIDER.wire_value();
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
                P::event_owner(event)
            );
            store.user_disable(P::event_owner(event).clone()).await?;
            Ok(Summary::default())
        }
        crate::failure::FailureDecision::Propagate => Err(err),
        crate::failure::FailureDecision::DropEvent => {
            warn!(
                "dropping poison event (owner: {}): {err:#}",
                P::event_owner(event)
            );
            store.event_delete(P::event_id(event)).await?;
            Ok(Summary::default())
        }
    }
}

#[cfg(test)]
mod queue_tests;
