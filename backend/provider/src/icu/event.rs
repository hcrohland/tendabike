//! The icu event queue row (the `icu_events` table, spec #763) and the
//! queue's read/process ops (spec #764).
//!
//! Mirrors the Strava module's `event` in shape: a queued webhook/sync event
//! the executor drains. `IcuEvent::accept` is the webhook acceptance (map the
//! athlete to a TendaBike user and queue the row); [`get_event`] is the
//! queue's read-side rule (the rate-limit gate, then the oldest queued
//! event, then the per-activity dedup — only the latest event per activity
//! is interesting, spec #996); [`process`] drains the queue in one call,
//! importing each activity, and carries the one stub-skip notice a batch
//! produces.

use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::activity::IcuActivity;
use super::http::request_json;
use super::{IcuId, IcuSpec};
use crate::EntityFacade;
use crate::facade::{Error, Summary, TbResult, UserId};
use crate::lane::QueueRead;
use crate::store::Store;

/// The `activity_id` sentinel a `SYNC` row carries (the column is `NOT NULL`,
/// but a `SYNC` refers to no single activity).
const SYNC_ACTIVITY_ID: &str = "sync";

/// The `icu_events` object type: a queued activity (a webhook row) or the
/// durable backfill cursor (`SYNC` — the row's `event_time` is the `newest`
/// cursor and its `oldest` column the walk's lower bound).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IcuObjectType {
    /// A queued activity (a webhook row).
    #[default]
    Activity,
    /// The durable backfill cursor.
    Sync,
}

impl TryFrom<String> for IcuObjectType {
    type Error = crate::facade::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        use IcuObjectType::*;
        Ok(match value.as_str() {
            "ACTIVITY" => Activity,
            "SYNC" => Sync,
            _ => {
                return Err(Error::BadRequest(format!(
                    "Unknown icu object type {value}"
                )));
            }
        })
    }
}

impl From<IcuObjectType> for String {
    fn from(value: IcuObjectType) -> Self {
        use IcuObjectType::*;
        String::from(match value {
            Activity => "ACTIVITY",
            Sync => "SYNC",
        })
    }
}

/// A queued icu event (a row in the `icu_events` table).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IcuEvent {
    /// The row id (`None` until it is stored).
    pub id: Option<i32>,
    /// The icu athlete the event belongs to.
    pub icu_athlete_id: IcuId,
    /// The activity the event refers to (the `"sync"` sentinel for a `SYNC`
    /// row, which refers to no single activity).
    pub activity_id: String,
    /// The event's unix time (the `newest` cursor for a `SYNC` row).
    pub event_time: i64,
    /// The event's object type (`ACTIVITY` | `SYNC`).
    pub object_type: IcuObjectType,
    /// The webhook event type (`ACTIVITY_UPLOADED` | `ACTIVITY_UPDATED` |
    /// `ACTIVITY_DELETED`) — the queue's dispatch key. Empty for a `SYNC` row
    /// (its dispatch is on `object_type`).
    pub event_type: String,
    /// The `SYNC` walk's fixed lower bound (unix time) — `None` for an
    /// `ACTIVITY` event, which has no walk.
    pub oldest: Option<i64>,
}

impl IcuEvent {
    /// A queued `ACTIVITY` event with no row id yet (a fresh webhook enqueue).
    /// `event_type` is the webhook's event type (the queue's dispatch key).
    pub fn new(
        icu_athlete_id: IcuId,
        activity_id: impl Into<String>,
        event_time: i64,
        event_type: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            icu_athlete_id,
            activity_id: activity_id.into(),
            event_time,
            object_type: IcuObjectType::Activity,
            event_type: event_type.into(),
            oldest: None,
        }
    }

    /// A queued `SYNC` event with no row id yet (the durable backfill cursor):
    /// the fixed `oldest` bound and the `0` `newest` sentinel
    /// (resolved to now at first processing). The `activity_id` carries the
    /// `"sync"` sentinel (the column is `NOT NULL`), and the dispatch is on
    /// `object_type`.
    pub fn new_sync(icu_athlete_id: IcuId, oldest: i64) -> Self {
        Self {
            id: None,
            icu_athlete_id,
            activity_id: SYNC_ACTIVITY_ID.to_string(),
            event_time: 0,
            object_type: IcuObjectType::Sync,
            event_type: String::new(),
            oldest: Some(oldest),
        }
    }

    /// Accepts a webhook event (spec #764): validates the athlete (mapping it
    /// to a TendaBike user), then queues the event. Returns the owner's
    /// TendaBike [`UserId`] (so the caller can wake that user).
    pub async fn accept(self, store: &mut impl Store<IcuSpec>) -> TbResult<UserId> {
        let user = self.icu_athlete_id.read(store).await?.ok_or_else(|| {
            Error::BadRequest(format!("Unknown icu athlete: {}", self.icu_athlete_id))
        })?;
        store.event_create(self).await?;
        Ok(user.tendabike_id)
    }
}

/// The intervals.icu webhook envelope (the `POST /icu/callback` body,
/// ADR-0014): the app's shared `secret` and the array of `events`.
#[derive(Debug, Deserialize)]
struct IcuWebhook {
    /// The app's shared webhook secret (verified against
    /// `ICU_WEBHOOK_SECRET`).
    secret: String,
    /// The events in the delivery (an intervals.icu app has one webhook URL
    /// for every athlete who authorized it, so a delivery can carry several).
    events: Vec<IcuWebhookEvent>,
}

/// One event in an intervals.icu webhook delivery.
#[derive(Debug, Deserialize)]
struct IcuWebhookEvent {
    /// The icu athlete the event belongs to.
    athlete_id: String,
    /// The event type (e.g. `ACTIVITY_UPLOADED`); only the subscribed activity
    /// types are imported.
    #[serde(rename = "type")]
    type_: String,
    /// The event's RFC 3339 timestamp.
    timestamp: String,
    /// The activity the event refers to (its id is the `icu_events` key).
    /// `None` for an event that carries no activity (a test delivery, an
    /// `APP_SCOPE_CHANGED`).
    activity: Option<IcuWebhookActivity>,
    /// Whether the app was fully revoked (`true`) or its scope reduced
    /// (`false`): present only on an `APP_SCOPE_CHANGED` event.
    deauthorized: Option<bool>,
    /// The app's scope after the change (empty on a full revocation):
    /// present only on an `APP_SCOPE_CHANGED` event.
    scope: Option<String>,
    /// Any field the app does not yet model (e.g. `client_id` on an
    /// `APP_SCOPE_CHANGED`): serde would otherwise drop it, so the skip log
    /// (`{event:?}`) would hide the part of the shape we most need to see.
    /// Read only through the derived `Debug` impl in that log; the leading
    /// underscore keeps the dead-code lint quiet (it does not count a derived
    /// `Debug` read as a use).
    #[serde(flatten)]
    _extra: serde_json::Map<String, serde_json::Value>,
}

/// The activity an intervals.icu activity event refers to.
#[derive(Debug, Deserialize)]
struct IcuWebhookActivity {
    id: String,
}

/// The activity event types the webhook subscribes to (spec #764; `ACTIVITY_DELETED`
/// for the delete path).
const SUBSCRIBED_ACTIVITY_TYPES: [&str; 3] =
    ["ACTIVITY_UPLOADED", "ACTIVITY_UPDATED", "ACTIVITY_DELETED"];

/// The webhook event type that deletes the activity: the queue's
/// `process` op dispatches on this — it calls the domain's `delete_activity`
/// instead of importing.
const DELETED_EVENT_TYPE: &str = "ACTIVITY_DELETED";

/// The `TEST` event type: intervals.icu fires a single `TEST` delivery (with
/// no activity) when the webhook is configured on the *Manage App* page.
const TEST_EVENT_TYPE: &str = "TEST";

/// The `APP_SCOPE_CHANGED` event type: intervals.icu fires it when a user
/// revokes the app or changes its scopes. Any scope change deauths (the user
/// is disabled inline at intake — no scope-content check).
const SCOPE_CHANGED_EVENT_TYPE: &str = "APP_SCOPE_CHANGED";

/// The shared webhook secret (from the provider config).
fn webhook_secret() -> String {
    crate::config::get().icu_webhook_secret.clone()
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
/// event filtering (only the subscribed activity types are imported, a
/// `TEST` delivery is logged and dropped, and an `APP_SCOPE_CHANGED`
/// delivery disables its user inline), the timestamp
/// parse, and the queue insert. `body` is the raw webhook body the
/// presentation layer forwards (no driver wire type crosses the seam).
/// Returns the TendaBike users a row was queued for (the caller's wake list).
pub async fn accept_event(body: &str, store: &mut impl Store<IcuSpec>) -> TbResult<Vec<UserId>> {
    let envelope: IcuWebhook = serde_json::from_str(body)
        .map_err(|e| Error::BadRequest(format!("invalid icu webhook envelope: {e}")))?;
    trace!("Received icu webhook with {} events", envelope.events.len());
    verify_secret(&envelope.secret)?;
    let mut queued = Vec::new();
    for event in &envelope.events {
        if event.type_ == TEST_EVENT_TYPE {
            info!("icu webhook test delivery: {event:?}");
            continue; // a test delivery (no activity) is not queueable
        }
        if event.type_ == SCOPE_CHANGED_EVENT_TYPE {
            handle_scope_change(event, store).await?;
            continue; // a scope change is handled inline: not queued, no wake
        }
        let Some(activity) = &event.activity else {
            debug!("Skipping icu webhook event (no activity): {event:?}");
            continue; // an event without an activity is not queueable
        };
        if !SUBSCRIBED_ACTIVITY_TYPES.contains(&event.type_.as_str()) {
            debug!("Skipping icu webhook event (not a subscribed type): {event:?}");
            continue; // only the subscribed activity types are imported (spec #764)
        }
        let icu_event = IcuEvent::new(
            IcuId::from(event.athlete_id.as_str()),
            activity.id.clone(),
            parse_timestamp(&event.timestamp)?,
            event.type_.clone(),
        );
        queued.push(icu_event.accept(store).await?);
    }
    Ok(queued)
}

/// Handles an `APP_SCOPE_CHANGED` event inline at intake (mirroring the
/// Strava driver's athlete-deauth handling): the known user is
/// disabled (their queued events dropped, their token cleared) with an INFO
/// log naming the athlete and the event's `deauthorized`/`scope` context; an
/// unknown athlete is a logged no-op. The event is neither queued nor added
/// to the wake list — the disable drops the queue, so there is nothing to
/// process.
async fn handle_scope_change(
    event: &IcuWebhookEvent,
    store: &mut impl Store<IcuSpec>,
) -> TbResult<()> {
    let icu_id = IcuId::from(event.athlete_id.as_str());
    match icu_id.read(store).await? {
        Some(_) => {
            info!(
                "icu scope change (deauthorized: {:?}, scope: {:?}): disabling icu user {icu_id}",
                event.deauthorized, event.scope
            );
            let res = icu_id.disable(store).await;
            if let Err(err) = res {
                error!("user disable returned: {err:#}")
            }
        }
        None => {
            info!("icu scope change for unknown athlete {icu_id}: no-op");
        }
    }
    Ok(())
}

/// The icu athlete id for a TendaBike user (the `icu_events` queue's key).
pub(crate) async fn get_icu_id(
    user_id: UserId,
    store: &mut impl Store<IcuSpec>,
) -> TbResult<IcuId> {
    let user = store.user_get_by_tbid(user_id).await?;
    Ok(user.id)
}

/// Enqueue (or replace) the user's `SYNC` event: the durable
/// backfill cursor. The `oldest` bound is fixed, and the `newest` sentinel is
/// `0` (resolved to now at first processing). **REPLACE** semantics — at most
/// one `SYNC` per user: a new enqueue drops the user's existing `SYNC` row (a
/// mid-walk cursor is discarded) before queueing the fresh one.
pub(crate) async fn insert_sync(
    user_id: UserId,
    oldest: i64,
    store: &mut impl Store<IcuSpec>,
) -> TbResult<()> {
    let icu_id = get_icu_id(user_id, store).await?;
    store.event_delete_sync(&icu_id).await?;
    let event = IcuEvent::new_sync(icu_id, oldest);
    store.event_create(event).await?;
    Ok(())
}

/// Dispatches one queued event to its domain op: an
/// `ACTIVITY_DELETED` event deletes the activity; every other subscribed event
/// imports it. Returns `Ok(None)` for a skipped stub (the import path only — a
/// delete is never a stub); errors (a 401, a 429, a DB failure) bubble up for
/// the batch to classify.
async fn dispatch_event<D: EntityFacade<S>, S: Store<IcuSpec>>(
    event: &IcuEvent,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<Summary>> {
    if event.event_type == DELETED_EVENT_TYPE {
        return Ok(Some(
            super::activity::delete_activity(&event.activity_id, user_id, facade, store).await?,
        ));
    }
    super::activity::import_activity(&event.activity_id, user_id, facade, store).await
}

/// Process the user's next queued icu event (the lane's `process`): read the
/// queue with the kit's read-side rules ([`crate::queue::get_event`]) and
/// handle only that one event (spec #1023 — symmetric with Strava: one event
/// per call, not a batch drain).
///
/// The event's `object_type` decides the path: a `SYNC` row runs one batch of
/// the durable backfill cursor walk ([`process_sync`]), and an `ACTIVITY` row
/// takes the import path. The queue's read-side rules (the rate-limit gate,
/// the latest-per-activity dedup) and the failure policy (a 429 keeps the
/// event queued, a 401 disables the user, a permanent error drops the event,
/// a DB failure propagates) are the kit's — the event is deleted in exactly
/// one place per path (no double-delete).
pub(crate) async fn process<D: EntityFacade<S>, S: Store<IcuSpec>>(
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    let icu_id = get_icu_id(user_id, store).await?;
    let read = crate::queue::get_event(icu_id, store).await?;
    let event = match read {
        QueueRead::Event(event) => event,
        // The queue is empty, or paused by a global rate-limit `Stop` (the
        // caller sleeps until its deadline; nothing to process).
        QueueRead::Limited { .. } | QueueRead::Empty => return Ok(Summary::default()),
    };

    match event.object_type {
        // A `SYNC` row: one batch of the durable cursor walk (the walk owns
        // the row's fate — delete on completion, stay queued on a 429).
        IcuObjectType::Sync => process_sync(event, user_id, facade, store).await,
        // An `ACTIVITY` row: the import path.
        IcuObjectType::Activity => {
            info!("Processing icu event: {}", event.activity_id);
            match dispatch_event(&event, user_id, facade, store).await {
                // A real activity: imported; the event is consumed.
                Ok(Some(summary)) => {
                    store.event_delete(event.id).await?;
                    Ok(summary)
                }
                // A stub (both `distance` and `moving_time` null-or-zero):
                // skipped (the gear reference is dropped, no upsert), with one
                // transient notice per event (spec #764; spec #1023 makes it
                // per-event).
                Ok(None) => {
                    let notice = super::activity::stub_notice(user_id, 1, facade, store).await?;
                    store.event_delete(event.id).await?;
                    let mut summary = Summary::default();
                    summary.notifications.upsert(notice);
                    Ok(summary)
                }
                // The kit's failure policy (spec #446 §4.6): a 429 keeps the
                // event queued, a 401 disables the user, a permanent error
                // drops the event (the delete is the kit's, in exactly one
                // place), a DB failure propagates.
                Err(err) => crate::queue::fail_event(&event, err, store).await,
            }
        }
    }
}

/// Format a unix timestamp as `yyyy-MM-dd` (the icu API's date param format).
fn date_str(ts: i64) -> String {
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap();
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day())
}

/// The batch size for each list fetch.
const LIMIT: i32 = 100;
/// The fields the domain Activity records (the `fields=` param).
const FIELDS: &str = "id,type,name,start_date,distance,moving_time,elapsed_time,total_elevation_gain,total_elevation_loss,gear,icu_joules";

/// The `SYNC` cursor walk, one batch per call (spec #1023 — strava's shape):
/// the durable, rate-limit-aware date-window walk that imports the user's
/// activities in batches of 100 (newest first).
///
/// One batch of up to [`LIMIT`] per call: the `0` `newest` sentinel resolves
/// to now on the first run (the cursor is persisted); after a successful full
/// batch the persisted `newest` cursor is advanced to the batch's oldest
/// activity, so the next `process` resumes from it. The row's fate per call:
///
/// - end of history (a `< 100` batch, an empty batch, or the cursor reaching
///   `oldest`) → the `SYNC` row is deleted and the batch's `Summary` returned
///   (it rides the lane's frame, so the imported activities reach the
///   frontend without a re-hydration);
/// - a full batch → the cursor is advanced (persisted) and the row stays
///   queued for the next `process`;
/// - a 429 / dead-auth / DB failure → the kit's [`fail_event`](
///   crate::queue::fail_event) policy (the 429 keeps the row queued at the
///   last persisted cursor — the cursor is *not* advanced past a 429 — the
///   401 disables the user, the DB failure propagates).
async fn process_sync<D: EntityFacade<S>, S: Store<IcuSpec>>(
    event: IcuEvent,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    match self::sync_batch(event.clone(), user_id, facade, store).await {
        Ok(summary) => Ok(summary),
        // The kit's failure policy (spec #446 §4.6): a 429 keeps the row
        // queued, a 401 disables the user (which drops the row), a DB failure
        // propagates.
        Err(err) => crate::queue::fail_event(&event, err, store).await,
    }
}

/// One batch of the `SYNC` cursor walk (the [`process_sync`] inner): fetch
/// one date-window batch of up to [`LIMIT`] activities, import it, and decide
/// the row's fate (delete on end-of-history, advance the persisted cursor on
/// a full batch). A 429 / dead-auth / DB failure bubbles up for
/// [`process_sync`] to classify.
async fn sync_batch<D: EntityFacade<S>, S: Store<IcuSpec>>(
    mut event: IcuEvent,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    let oldest = event.oldest.unwrap_or(0);
    let mut cursor = event.event_time;
    // The `0` sentinel: resolve to now on the first run (persist the cursor).
    if cursor == 0 {
        cursor = OffsetDateTime::now_utc().unix_timestamp();
        event.event_time = cursor;
        store.event_set_time(event.id, cursor).await?;
    }
    // The cursor reached the lower bound: end of history (the row is done).
    if cursor <= oldest {
        store.event_delete(event.id).await?;
        return Ok(Summary::default());
    }
    // The proactive rate-limit gate: budget exhausted → do not even make the
    // call; bubble a 429 for the failure policy (which keeps the row queued
    // at the last persisted cursor).
    if super::ratelimit::limited_until().is_some() {
        return Err(Error::TryAgain(
            "icu sync rate limited; the cursor stays in place",
        ));
    }
    let activities: Vec<IcuActivity> = request_json(
        user_id,
        &format!(
            "/athlete/{}/activities?oldest={}&newest={}&limit={LIMIT}&fields={FIELDS}",
            event.icu_athlete_id,
            date_str(oldest),
            date_str(cursor)
        ),
        store,
    )
    .await?;
    if activities.is_empty() {
        // No activities in the window: end of history (the row is done).
        store.event_delete(event.id).await?;
        return Ok(Summary::default());
    }
    info!(
        "icu sync: {} → {} activities",
        date_str(cursor),
        activities.len()
    );
    let count = activities.len();
    // The API returns in descending order: the last element is the oldest.
    let oldest_in_batch = activities.last().unwrap().start_date.unix_timestamp();
    let mut summary = Summary::default();
    for act in activities {
        if let Some(s) = super::activity::import_activity_data(act, user_id, facade, store).await? {
            summary += s;
        }
    }
    if count < LIMIT as usize {
        // A short batch: end of history (the row is done).
        store.event_delete(event.id).await?;
        return Ok(summary);
    }
    // A full batch: advance the persisted cursor to the batch's oldest
    // activity (the date-only API precision — the same day, so the remaining
    // same-day activities get fetched next). A full batch that did not clear
    // the start day (the oldest activity is still on it) steps a whole day
    // back instead: re-querying the same day would loop.
    let next = if date_str(oldest_in_batch) == date_str(cursor) {
        error!(
            "icu sync: a full batch did not clear the start day {}; stepping a day back",
            date_str(cursor)
        );
        cursor - 86400
    } else {
        oldest_in_batch
    };
    if next <= oldest {
        // The advanced cursor reached the lower bound: end of history (the
        // row is done).
        store.event_delete(event.id).await?;
    } else {
        // The row stays queued: the next `process` resumes from the cursor.
        event.event_time = next;
        store.event_set_time(event.id, next).await?;
    }
    Ok(summary)
}

#[cfg(test)]
mod event_tests;
