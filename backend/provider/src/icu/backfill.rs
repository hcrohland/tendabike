//! The icu onboarding backfill (spec #766): the date-cursor walk that imports
//! the user's past activities directly (no queue, no per-activity fetch).
//!
//! On first connect the user's history is imported automatically. The walk
//! reaches back [`BACKFILL_MONTHS`] months, fetching batches via
//! `GET /athlete/{id}/activities?oldest=&limit=&fields=` and importing each
//! activity into the domain (through [`super::activity::import_activity`]).
//!
//! The walk is **resumable**: the cursor is the newest queued
//! `icu_events.event_time`. A fresh onboarding starts at `now - 6 months`; a
//! resume starts at the last imported activity's time.
//!
//! The walk is **rate-limit aware**: it stops when the budget is exhausted
//! (the proactive gate) or a fetch is 429'd. The activities imported so far
//! are committed, and a later call resumes from the newest imported time.

use time::OffsetDateTime;

use super::IcuDataStore;
use super::activity::import_activity_data;
use super::http::request_json;
use tb_domain::{Error, TbResult, UserId};

/// Format a unix timestamp as `yyyy-MM-dd` (the icu API's date param format).
fn date_str(ts: i64) -> String {
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap();
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day())
}

/// How far back the onboarding backfill reaches (the "oldest desired date").
const BACKFILL_MONTHS: i64 = 120;
/// A one-month span, in seconds (the backfill's reach, not a window size).
const MONTH: i64 = 30 * 24 * 3600;
/// The batch size for each list fetch.
const LIMIT: i32 = 100;
/// The fields the domain Activity records (the `fields=` param).
const FIELDS: &str = "id,type,name,start_date,distance,moving_time,elapsed_time,total_elevation_gain,total_elevation_loss,gear,icu_joules";

/// One activity in a backfill batch (`GET /athlete/{id}/activities`): the
/// fields the domain Activity records.
#[derive(serde::Deserialize)]
struct BackfillActivity {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    start_date: OffsetDateTime,
    distance: Option<f64>,
    moving_time: Option<i32>,
    elapsed_time: i32,
    total_elevation_gain: Option<f64>,
    total_elevation_loss: Option<f64>,
    gear: Option<super::activity::IcuGearRef>,
    icu_joules: Option<f64>,
}

/// Walk the user's activity history and import each activity directly into
/// the domain (the onboarding backfill, spec #766).
///
/// `now` is the backfill's "now" (the onboarding time the cutover passes).
/// The cursor is the newest imported `icu_events.event_time` (the resume
/// point): a fresh onboarding starts at `now - 6 months`, a resume starts at
/// the last imported activity's time. Each batch is fetched via
/// `GET /athlete/{id}/activities?oldest=&limit=&fields=` and its activities
/// are imported one at a time.
///
/// Rate-limit aware: the walk stops when the budget is exhausted (a 429, or
/// the proactive gate) — the activities imported so far commit, and a later
/// call resumes from the newest imported time.
pub(crate) async fn backfill<D: crate::DomainFacade<S>, S: IcuDataStore>(
    user_id: UserId,
    now: i64,
    facade: &D,
    store: &mut S,
) -> TbResult<()> {
    let icu_id = super::event::get_icu_id(user_id, store).await?;
    let oldest_desired = now - BACKFILL_MONTHS * MONTH;
    // The resume cursor: the newest imported event's time, or the oldest
    // desired date for a fresh onboarding.
    let start = store
        .icu_events_get_latest_time(&icu_id)
        .await?
        .unwrap_or(oldest_desired);

    let mut oldest = start;
    while oldest < now {
        // Proactive rate-limit gate: budget exhausted → stop.
        if super::ratelimit::limited_until().is_some() {
            break;
        }
        let activities: Vec<BackfillActivity> = match request_json(
            user_id,
            &format!(
                "/athlete/{icu_id}/activities?oldest={}&limit={LIMIT}&fields={FIELDS}",
                date_str(oldest)
            ),
            store,
        )
        .await
        {
            Ok(activities) => activities,
            // A 429: stop the walk (the imported activities commit; a later
            // call resumes from the newest imported time).
            Err(Error::TryAgain(msg)) => {
                log::warn!(
                    "icu backfill rate limited ({msg}); resuming from the newest imported activity"
                );
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        if activities.is_empty() {
            break;
        }
        log::debug!(
            "icu backfill: from {} → {} activities",
            date_str(oldest),
            activities.len()
        );
        let mut last_time = oldest;
        let count = activities.len();
        for a in activities {
            // Import the activity directly (no queue, no second fetch).
            let icu_act = super::activity::IcuActivity {
                id: a.id,
                type_: a.type_,
                name: a.name,
                start_date: a.start_date,
                distance: a.distance,
                moving_time: a.moving_time,
                elapsed_time: a.elapsed_time,
                total_elevation_gain: a.total_elevation_gain,
                total_elevation_loss: a.total_elevation_loss,
                gear: a.gear,
                icu_joules: a.icu_joules,
            };
            let _summary = import_activity_data(icu_act, user_id, facade, store).await?;
            last_time = a.start_date.unix_timestamp().max(last_time);
        }
        // Advance the cursor past the last imported activity.
        if count < LIMIT as usize {
            break; // End of history.
        }
        oldest = last_time + 1;
    }
    Ok(())
}

#[cfg(test)]
mod backfill_tests;
