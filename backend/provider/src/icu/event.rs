//! The icu event queue row (the `icu_events` table, spec #763) and the
//! queue's read/process ops (spec #764).
//!
//! Mirrors the Strava module's `event` in shape: a queued webhook/sync event
//! the executor drains. `IcuEvent::accept` is the webhook acceptance (map the
//! athlete to a TendaBike user and queue the row); [`get_event`] is the
//! queue's read-side rule (the rate-limit gate, then the oldest queued
//! event); [`process`] drains the queue in one call, importing each activity,
//! and carries the one stub-skip notice a batch produces.

use log::{info, warn};
use serde::{Deserialize, Serialize};

use super::{IcuDataStore, IcuId};
use crate::DomainFacade;
use tb_domain::{Error, Summary, TbResult, UserId};

/// A queued icu event (a row in the `icu_events` table).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IcuEvent {
    /// The row id (`None` until it is stored).
    pub id: Option<i32>,
    /// The icu athlete the event belongs to.
    pub icu_athlete_id: IcuId,
    /// The activity the event refers to.
    pub activity_id: String,
    /// The event's unix time (also the backfill cursor).
    pub event_time: i64,
    /// The event's object type (e.g. `ACTIVITY`).
    pub object_type: String,
}

impl IcuEvent {
    /// A queued icu event with no row id yet (a fresh webhook/sync enqueue).
    pub fn new(icu_athlete_id: IcuId, activity_id: impl Into<String>, event_time: i64) -> Self {
        Self {
            id: None,
            icu_athlete_id,
            activity_id: activity_id.into(),
            event_time,
            object_type: "ACTIVITY".to_string(),
        }
    }

    /// Accepts a webhook event (spec #764): validates the athlete (mapping it
    /// to a TendaBike user), then queues the event. Returns the owner's
    /// TendaBike [`UserId`] (so the caller can wake that user).
    pub async fn accept(self, store: &mut impl IcuDataStore) -> TbResult<UserId> {
        let user = self.icu_athlete_id.read(store).await?.ok_or_else(|| {
            Error::BadRequest(format!("Unknown icu athlete: {}", self.icu_athlete_id))
        })?;
        store.icu_event_create(self).await?;
        Ok(user.tendabike_id)
    }
}

/// The icu athlete id for a TendaBike user (the `icu_events` queue's key).
pub(crate) async fn get_icu_id(user_id: UserId, store: &mut impl IcuDataStore) -> TbResult<IcuId> {
    let user = store.icuser_get_by_tbid(user_id).await?;
    Ok(user.id)
}

/// One outcome of a read of the `icu_events` queue (spec #764): what is
/// interesting for the user right now, and when the queue is paused.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QueueRead {
    /// An event to process (the next queued row).
    Event(IcuEvent),
    /// The icu rate-limit budget is exhausted until this unix time: the queue
    /// is paused, and the caller sleeps until the window boundary.
    Limited { until: i64 },
    /// The queue holds nothing for this user.
    Empty,
}

/// Read the queue for the user, with the queue's read-side rules (the
/// rate-limit gate, then the oldest queued event). Crate-internal: the lane's
/// `probe` is the only caller.
pub(crate) async fn get_event(
    user_id: UserId,
    store: &mut impl IcuDataStore,
) -> TbResult<QueueRead> {
    // Proactive rate-limit gate: budget exhausted → report the window boundary
    // without dequeuing (the event stays queued).
    if let Some(until) = super::ratelimit::limited_until() {
        return Ok(QueueRead::Limited { until });
    }
    let icu_id = get_icu_id(user_id, store).await?;
    match store.icu_event_get_next_for_user(icu_id).await? {
        Some(event) => Ok(QueueRead::Event(event)),
        None => Ok(QueueRead::Empty),
    }
}

/// Drains the user's `icu_events` queue in one call (the lane's `process`),
/// importing each activity into the domain (spec #764).
///
/// The batch is bounded by the queue, the rate-limit gate, and the failure
/// classification:
///
/// - **stub** (both `distance` and `moving_time` null-or-zero) → skipped (the
///   gear reference is dropped, no upsert); the count feeds the one batch
///   notice at the end.
/// - **429 / rate-limited** → stop; the current event stays queued (the probe
///   reports `Limited` and the loop sleeps until the window boundary).
/// - **401 / dead auth** → disable the user (drops all their queued events).
/// - **permanent** (a 404, a parse error) → drop the event and continue.
/// - **DB failure** → propagate: the caller rolls back and backs off.
pub(crate) async fn process<D: DomainFacade<S>, S: IcuDataStore>(
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    let icu_id = get_icu_id(user_id, store).await?;
    let mut summary = Summary::default();
    let mut stubs = 0usize;

    loop {
        // The proactive rate-limit gate: budget exhausted → stop (the current
        // and remaining events stay queued; the probe reports `Limited`).
        if super::ratelimit::limited_until().is_some() {
            break;
        }
        let Some(event) = store.icu_event_get_next_for_user(icu_id.clone()).await? else {
            break; // the queue is empty
        };
        info!("Processing icu event: {}", event.activity_id);
        match super::activity::import_activity(&event.activity_id, user_id, facade, store).await {
            Ok(Some(s)) => summary += s,
            Ok(None) => stubs += 1, // a stub: skipped (gear ref dropped)
            Err(err) => match classify(err, &event, &icu_id, store).await? {
                BatchOutcome::Stop => break, // 429 (event stays queued) or 401 (user disabled)
                BatchOutcome::Continue => {} // permanent: the event was dropped, continue
            },
        }
        store.icu_event_delete(event.id).await?;
    }

    // One transient notice per batch with the skip count (spec #764).
    if stubs > 0 {
        let notice = super::activity::stub_notice(user_id, stubs, facade, store).await?;
        summary.notifications.upsert(notice);
    }
    Ok(summary)
}

/// The outcome of a failed import in a `process` batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchOutcome {
    /// Stop the batch (a 429 — the event stays queued — or a 401 — the user
    /// is disabled, which drops all their queued events).
    Stop,
    /// The event was permanent-bad and dropped; continue the batch.
    Continue,
}

/// Classifies a failed import (the batch's failure semantics, spec #764).
///
/// - `TryAgain` (429) → stop; the event stays queued (the probe reports
///   `Limited`).
/// - `NotAuth` (401) → disable the user (drops all their queued events); stop.
/// - `DatabaseFailure` → propagate (the caller rolls back and backs off).
/// - anything else is permanent (a 404, a parse) → drop the event; continue.
async fn classify(
    err: Error,
    event: &IcuEvent,
    icu_id: &IcuId,
    store: &mut impl IcuDataStore,
) -> TbResult<BatchOutcome> {
    match err {
        Error::TryAgain(msg) => {
            warn!("icu rate limited ({msg}); event stays queued until the window resets");
            Ok(BatchOutcome::Stop)
        }
        Error::NotAuth(msg) => {
            warn!("icu auth is dead ({msg}); disabling user {icu_id}");
            icu_id.disable(store).await?;
            Ok(BatchOutcome::Stop)
        }
        Error::DatabaseFailure(_) => Err(err),
        other => {
            warn!("dropping poison icu event {}: {other:#}", event.activity_id);
            store.icu_event_delete(event.id).await?;
            Ok(BatchOutcome::Continue)
        }
    }
}

#[cfg(test)]
mod event_tests;
