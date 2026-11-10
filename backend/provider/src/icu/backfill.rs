//! The icu onboarding backfill (spec #766): the date-window walk that queues
//! the user's past activities as `icu_events` rows.
//!
//! On first connect the user's history is imported automatically. The walk
//! reaches back [`BACKFILL_MONTHS`] months in one-month windows, fetching each
//! window via `GET /athlete/{id}/activities?oldest=&newest=` and queueing its
//! activities as `icu_events` rows (`object_type` `ACTIVITY_ANALYZED`,
//! `event_time` the activity's start time). The normal event loop
//! ([`super::event::process`]) then imports them one at a time — there is no
//! special backfill processing path.
//!
//! The walk is **resumable**: the cursor is the newest queued
//! `icu_events.event_time`. A fresh onboarding starts at `now - 6 months`; a
//! resume (the process died mid-walk) starts at the last queued event's time,
//! so the windows already walked are not re-fetched.
//!
//! The walk is **rate-limit aware**: it stops when the budget is exhausted
//! (the proactive gate) or a fetch is 429'd. The events queued so far commit,
//! and a later call resumes from the newest queued time (the loop's own
//! sleep-until-`Retry-After` covers the queued events).

use time::OffsetDateTime;

use super::http::request_json;
use super::{IcuDataStore, IcuEvent};
use tb_domain::{Error, TbResult, UserId};

/// A one-month backfill window, in seconds. The icu API's `oldest`/`newest`
/// are unix seconds; a fixed 30-day window keeps the walk simple and
/// resumable (the exact calendar month is immaterial to the cursor).
const MONTH: i64 = 30 * 24 * 3600;
/// How far back the onboarding backfill reaches (the "oldest desired date").
const BACKFILL_MONTHS: i64 = 6;
/// The backfill event's object type (the same the webhook path uses).
const ACTIVITY_ANALYZED: &str = "ACTIVITY_ANALYZED";

/// One activity in a backfill window (`GET /athlete/{id}/activities`): the
/// fields the queue row needs (the id and the start time).
#[derive(serde::Deserialize)]
struct BackfillActivity {
    id: String,
    #[serde(with = "time::serde::rfc3339")]
    start_date: OffsetDateTime,
}

/// Walk the user's activity history in one-month windows and queue each
/// activity as an `icu_events` row (the onboarding backfill, spec #766).
///
/// `now` is the backfill's "now" (the onboarding time the cutover passes).
/// The cursor is the newest queued `icu_events.event_time` (the resume point):
/// a fresh onboarding starts at `now - 6 months`, a resume starts at the last
/// queued event's time. Each window is fetched via
/// `GET /athlete/{id}/activities?oldest=&newest=` and its activities are
/// queueed (`object_type` `ACTIVITY_ANALYZED`, `event_time` the activity's
/// start time) for the normal event loop to import one at a time.
///
/// Rate-limit aware: the walk stops when the budget is exhausted (a 429, or
/// the proactive gate) — the events queued so far commit, and a later call
/// resumes from the newest queued time.
pub(crate) async fn backfill(
    user_id: UserId,
    now: i64,
    store: &mut impl IcuDataStore,
) -> TbResult<()> {
    let icu_id = super::event::get_icu_id(user_id, store).await?;
    let oldest_desired = now - BACKFILL_MONTHS * MONTH;
    // The resume cursor: the newest queued event's time, or the oldest
    // desired date for a fresh onboarding.
    let start = store
        .icu_events_get_latest_time(&icu_id)
        .await?
        .unwrap_or(oldest_desired);

    let mut oldest = start;
    while oldest < now {
        // Proactive rate-limit gate: budget exhausted → stop (the queued
        // events commit; a later call resumes from the newest queued time).
        if super::ratelimit::limited_until().is_some() {
            break;
        }
        let newest = (oldest + MONTH).min(now);
        let activities: Vec<BackfillActivity> = match request_json(
            user_id,
            &format!("/athlete/{icu_id}/activities?oldest={oldest}&newest={newest}"),
            store,
        )
        .await
        {
            Ok(activities) => activities,
            // A 429: stop the walk (the queued events commit; a later
            // call resumes from the newest queued time).
            Err(Error::TryAgain(msg)) => {
                log::warn!(
                    "icu backfill rate limited ({msg}); resuming from the newest queued event"
                );
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        for a in activities {
            let event = IcuEvent {
                id: None,
                icu_athlete_id: icu_id.clone(),
                activity_id: a.id,
                event_time: a.start_date.unix_timestamp(),
                object_type: ACTIVITY_ANALYZED.to_string(),
            };
            store.icu_event_create(event).await?;
        }
        oldest = newest;
    }
    Ok(())
}

#[cfg(test)]
mod backfill_tests;
