//! The icu event queue row (the `icu_events` table, spec #763) and the
//! queue's read/process ops (spec #764).
//!
//! Mirrors the Strava module's `event` in shape: a queued webhook/sync event
//! the executor drains. `IcuEvent::accept` is the webhook acceptance (map the
//! athlete to a TendaBike user and queue the row); [`get_event`] is the
//! queue's read-side rule (the rate-limit gate, then the oldest queued
//! event); [`process`] drains the queue in one call, importing each activity,
//! and carries the one stub-skip notice a batch produces.

use log::{info, trace, warn};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{IcuDataStore, IcuId};
use crate::EntityFacade;
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

/// The intervals.icu webhook envelope (the `POST /icu/callback` body,
/// ADR-0014): the app's shared `secret` and the array of `events`.
#[derive(Debug, Deserialize)]
pub struct IcuWebhook {
    /// The app's shared webhook secret (verified against
    /// `ICU_WEBHOOK_SECRET`).
    secret: String,
    /// The events in the delivery (an intervals.icu app has one webhook URL
    /// for every athlete who authorized it, so a delivery can carry several).
    events: Vec<IcuWebhookEvent>,
}

/// One event in an intervals.icu webhook delivery.
#[derive(Debug, Deserialize)]
pub struct IcuWebhookEvent {
    /// The icu athlete the event belongs to.
    athlete_id: String,
    /// The event type (e.g. `ACTIVITY_ANALYZED`); only `ACTIVITY_ANALYZED` is
    /// subscribed.
    #[serde(rename = "type")]
    type_: String,
    /// The event's RFC 3339 timestamp.
    timestamp: String,
    /// The activity the event refers to (its id is the `icu_events` key).
    activity: IcuWebhookActivity,
}

/// The activity an intervals.icu activity event refers to.
#[derive(Debug, Deserialize)]
pub struct IcuWebhookActivity {
    id: String,
}

/// The `ACTIVITY_ANALYZED` event type: the only event the webhook subscribes
/// to (spec #764).
const ACTIVITY_ANALYZED: &str = "ACTIVITY_ANALYZED";

/// The shared webhook secret (the app's intervals.icu webhook secret, the
/// `ICU_WEBHOOK_SECRET` env var).
fn webhook_secret() -> String {
    std::env::var("ICU_WEBHOOK_SECRET").unwrap_or_default()
}

/// A constant-time byte equality (the webhook secret check, spec #764): avoids
/// a timing side-channel on the shared secret. A length mismatch is not secret
/// (the secret's length is fixed per app), so it short-circuits.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify the webhook's shared secret (spec #764): a constant-time compare
/// against `ICU_WEBHOOK_SECRET`. A mismatch (or an unset secret) is a `401`.
fn verify_secret(secret: &str) -> TbResult<()> {
    let expected = webhook_secret();
    if !constant_time_eq(secret.as_bytes(), expected.as_bytes()) {
        return Err(Error::NotAuth("invalid icu webhook secret".to_string()));
    }
    Ok(())
}

/// Parse an intervals.icu webhook timestamp (RFC 3339) to a unix second.
fn parse_timestamp(timestamp: &str) -> TbResult<i64> {
    OffsetDateTime::parse(timestamp, &Rfc3339)
        .map(|dt| dt.unix_timestamp())
        .map_err(|_| Error::BadRequest(format!("invalid icu webhook timestamp {timestamp}")))
}

/// The webhook intake (ADR-0014): the provider's webhook work behind the
/// seam — the envelope parse of the raw body, the secret verification, the
/// event filtering (only `ACTIVITY_ANALYZED` is subscribed), the timestamp
/// parse, and the queue insert. `body` is the raw webhook body the
/// presentation layer forwards (no driver wire type crosses the seam).
/// Returns the TendaBike users a row was queued for (the caller's wake list).
pub async fn accept_event(body: &str, store: &mut impl IcuDataStore) -> TbResult<Vec<UserId>> {
    let envelope: IcuWebhook = serde_json::from_str(body)
        .map_err(|e| Error::BadRequest(format!("invalid icu webhook envelope: {e}")))?;
    trace!("Received icu webhook with {} events", envelope.events.len());
    verify_secret(&envelope.secret)?;
    let mut queued = Vec::new();
    for event in &envelope.events {
        if event.type_ != ACTIVITY_ANALYZED {
            continue; // only ACTIVITY_ANALYZED is subscribed (spec #764)
        }
        let icu_event = IcuEvent::new(
            IcuId::from(event.athlete_id.as_str()),
            event.activity.id.clone(),
            parse_timestamp(&event.timestamp)?,
        );
        queued.push(icu_event.accept(store).await?);
    }
    Ok(queued)
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
pub(crate) async fn process<D: EntityFacade<S>, S: IcuDataStore>(
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
            Err(err) => match apply_failure(err, &event, &icu_id, store).await? {
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

/// Applies the kit's failure decision ([`crate::failure`], the batch's
/// failure semantics, spec #764) to icu's tables: the disable and the drop
/// commit with the batch's transaction.
async fn apply_failure(
    err: Error,
    event: &IcuEvent,
    icu_id: &IcuId,
    store: &mut impl IcuDataStore,
) -> TbResult<BatchOutcome> {
    match crate::failure::classify(&err) {
        crate::failure::FailureDecision::KeepQueued => {
            warn!("icu rate limited ({err}); event stays queued until the window resets");
            Ok(BatchOutcome::Stop)
        }
        crate::failure::FailureDecision::DisableUser => {
            warn!("icu auth is dead ({err}); disabling user {icu_id}");
            icu_id.disable(store).await?;
            Ok(BatchOutcome::Stop)
        }
        crate::failure::FailureDecision::Propagate => Err(err),
        crate::failure::FailureDecision::DropEvent => {
            warn!("dropping poison icu event {}: {err:#}", event.activity_id);
            store.icu_event_delete(event.id).await?;
            Ok(BatchOutcome::Continue)
        }
    }
}

#[cfg(test)]
mod event_tests;
