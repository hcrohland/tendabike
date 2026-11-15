//! The icu activity backfill (spec #766): the date-cursor walk that imports
//! the user's activities directly (no queue, no per-activity fetch).
//!
//! The walk proceeds **backward** from `newest` to `oldest`: each batch
//! fetches the newest 100 activities in the window via
//! `GET /athlete/{id}/activities?oldest=&newest=&limit=&fields=`, and the
//! cursor advances to the day before the oldest activity in the batch. This
//! works with the intervals.icu API's descending-order results.
//!
//! Two callers use different bounds:
//! - **Onboarding**: `oldest = time - 240 months`, `newest = time`
//! - **Admin sync**: `oldest = event_time`, `newest = system now`
//!
//! The walk is **rate-limit aware**: it stops when the budget is exhausted
//! (the proactive gate) or a fetch is 429'd. The activities imported so far
//! are committed, and a later call resumes from the newest imported time.

use time::OffsetDateTime;

use super::IcuDataStore;
use super::activity::{IcuActivity, import_activity_data};
use super::http::request_json;
use crate::facade::{Error, TbResult, UserId};

/// Format a unix timestamp as `yyyy-MM-dd` (the icu API's date param format).
fn date_str(ts: i64) -> String {
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap();
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day())
}

/// How far back the onboarding backfill reaches (the "oldest desired date").
pub(crate) const BACKFILL_MONTHS: i64 = 240;
/// A one-month span, in seconds (the backfill's reach, not a window size).
pub(crate) const MONTH: i64 = 30 * 24 * 3600;
/// The batch size for each list fetch.
const LIMIT: i32 = 100;
/// The fields the domain Activity records (the `fields=` param).
const FIELDS: &str = "id,type,name,start_date,distance,moving_time,elapsed_time,total_elevation_gain,total_elevation_loss,gear,icu_joules";

/// Walk the user's activity history in `[oldest, newest]` and import each
/// activity directly into the domain (spec #766).
///
/// The walk proceeds **backward** from `newest` to `oldest`: each batch
/// fetches the newest 100 activities in the window via
/// `GET /athlete/{id}/activities?oldest=&newest=&limit=&fields=`, and the
/// cursor advances to the day before the oldest activity in the batch. The
/// walk stops when a batch returns fewer than 100 activities (end of
/// history), when the cursor reaches `oldest`, or when the rate-limit budget
/// is exhausted.
///
/// Rate-limit aware: the walk stops when the budget is exhausted (a 429, or
/// the proactive gate) — the activities imported so far commit, and a later
/// call resumes from the newest imported time.
pub(crate) async fn backfill<D: crate::EntityFacade<S>, S: IcuDataStore>(
    user_id: UserId,
    oldest: i64,
    newest: i64,
    facade: &D,
    store: &mut S,
) -> TbResult<()> {
    let icu_id = super::event::get_icu_id(user_id, store).await?;

    let mut cursor = newest;
    while cursor > oldest {
        // Proactive rate-limit gate: budget exhausted → stop.
        if super::ratelimit::limited_until().is_some() {
            break;
        }
        let activities: Vec<IcuActivity> = match request_json(
            user_id,
            &format!(
                "/athlete/{icu_id}/activities?oldest={}&newest={}&limit={LIMIT}&fields={FIELDS}",
                date_str(oldest),
                date_str(cursor)
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
            "icu backfill: to {} → {} activities",
            date_str(cursor),
            activities.len()
        );
        let count = activities.len();
        // The API returns in descending order: the last element is the oldest.
        let oldest_in_batch = activities.last().unwrap().start_date;
        for act in activities {
            let _summary = import_activity_data(act, user_id, facade, store).await?;
        }
        // Advance the cursor to the day before the oldest activity in this
        // batch (the date-only API precision means we step by whole days).
        if count < LIMIT as usize {
            break; // End of history.
        }
        cursor = (oldest_in_batch - time::Duration::days(1)).unix_timestamp();
    }
    Ok(())
}

#[cfg(test)]
mod backfill_tests;
