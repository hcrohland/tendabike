//! The icu event queue row (the `icu_events` table, spec #763).
//!
//! Mirrors the Strava module's [`crate::strava::event::Event`] in shape: a
//! queued webhook/sync event the executor drains one at a time. The
//! `event_time` doubles as the backfill cursor (a resumed backfill picks up
//! from the last queued event's time).

use serde::{Deserialize, Serialize};

use super::IcuId;

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
}
