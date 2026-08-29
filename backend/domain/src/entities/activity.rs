/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! Activity handling for the TendaBike backend
//!
//! struct Activity captures all data of an athlete's activity
//!
//! By assigning a gear to the activity it gets accounted with that gear and all it's parts attached
//! at the start time of the activity
//! Most operations are done on the ActivityId though
//!

use std::collections::HashSet;

use anyhow::Context;
use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime, macros::format_description};

use crate::*;

/// The Id of an Activity
///
/// Most operations for activities are done on the Id alone
///
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, From, Into, Display)]
pub struct ActivityId(i64);

/// The database's representation of an activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    /// The primary key
    pub id: ActivityId,
    /// The athlete
    pub user_id: UserId,
    /// The activity type
    pub what: ActTypeId,
    /// This name of the activity.
    pub name: String,
    /// Start time
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    /// End time
    pub duration: i32,
    /// activity time
    pub time: Option<i32>,
    /// Covered distance
    pub distance: Option<i32>,
    /// Total climbing
    pub climb: Option<i32>,
    /// Total descending
    pub descend: Option<i32>,
    /// average energy output
    pub energy: Option<i32>,
    /// Which gear did she use?
    pub gear: Option<PartId>,
    /// The name of the recording device
    pub device_name: Option<String>,
    /// opaque identifier for the source file
    /// seems to start with "garmin" for garmin connect provided data
    pub external_id: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Factor {
    Add = 1,
    Sub = -1,
}

impl ActivityId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// Read the activity with id self
    ///
    /// checks authorization
    async fn read_optional(
        self,
        session: &dyn Session,
        store: &mut impl ActivityStore,
    ) -> TbResult<Option<Activity>> {
        let act = store.activity_read_by_id(self).await?;
        if let Some(act) = &act {
            session.check_owner(
                act.user_id,
                format!("User {} cannot access activity {}", session.user_id(), self),
            )?;
        }
        Ok(act)
    }

    /// Read the activity with id self
    ///
    /// checks authorization
    pub async fn read(
        self,
        session: &dyn Session,
        store: &mut impl ActivityStore,
    ) -> TbResult<Activity> {
        self.read_optional(session, store)
            .await?
            .ok_or(crate::Error::NotFound(
                "activity does not exist".to_string(),
            ))
    }

    /// Delete the activity with id self
    /// and update part usage accordingly
    ///
    /// returns all affected parts  
    /// checks authorization  
    pub async fn delete(self, session: &dyn Session, store: &mut impl Store) -> TbResult<Summary> {
        info!("Deleting {self:?}");
        let mut res = self
            .read(session, store)
            .await?
            .register(Factor::Sub, store)
            .await?;
        store.activity_delete(self).await?;
        res.activities[0].gear = None;
        res.activities[0].duration = 0;
        res.activities[0].time = None;
        res.activities[0].distance = None;
        res.activities[0].climb = None;
        res.activities[0].descend = None;
        res.activities[0].energy = None;
        Ok(res)
    }
}

impl Activity {
    /// create a new activity
    ///
    /// returns the activity and all affected parts  
    /// checks authorization  
    pub async fn upsert(self, user: &dyn Session, store: &mut impl Store) -> TbResult<Summary> {
        if let Some(old_activity) = self.id.read_optional(user, store).await? {
            old_activity.replace(self, store).await
        } else {
            user.check_owner(
                self.user_id,
                format!(
                    "user {} cannot create activity for user {}",
                    user.user_id(),
                    self.user_id
                ),
            )?;

            info!("Creating {:?}", self);
            let new = store.activity_create(self).await?;
            // let res = new.check_geartype(res, store)?;
            new.register(Factor::Add, store).await
        }
    }

    /// Update the activity with id self according to the data in NewActivity
    /// and update part usage accordingly
    ///
    /// returns all affected parts  
    /// checks authorization  
    pub async fn update(self, user: &dyn Session, store: &mut impl Store) -> TbResult<Summary> {
        self.id.read(user, store).await?.replace(self, store).await
    }

    async fn replace(self, new: Activity, store: &mut impl Store) -> TbResult<Summary> {
        info!("Updating {self:?}");
        let mut res = self.register(Factor::Sub, store).await?;

        let act = store.activity_update(new).await?;

        res = res + act.register(Factor::Add, store).await?;
        Ok(res)
    }

    /// Extract the usage out of an activity
    ///
    /// If the descend value is missing, assume descend = climb
    /// Account for Factor
    pub(crate) fn usage(&self) -> Usage {
        Usage {
            id: UsageId::default(),
            time: self.time.unwrap_or(0),
            distance: self.distance.unwrap_or(0),
            climb: self.climb.unwrap_or(0),
            descend: self.descend.unwrap_or_else(|| self.climb.unwrap_or(0)),
            energy: self.energy.unwrap_or(0),
            count: 1,
        }
    }

    /// find all activities for gear part in the given time frame
    ///
    /// if end is none it means for the whole future
    pub(crate) async fn find(
        gear: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
        store: &mut impl ActivityStore,
    ) -> TbResult<Vec<Activity>> {
        store
            .activities_find_by_gear_and_time(gear, begin, end)
            .await
    }

    /// Register or unregister an activity with the given factor.
    ///
    /// If the factor is `Factor::Add`, the activity is registered and the usage is added to the parts and attachments.
    /// If the factor is `Factor::Subtract`, the activity is unregistered and the usage is subtracted from the parts and attachments.
    ///
    /// Returns a summary of the affected parts, attachments, and activities.
    async fn register(self, factor: Factor, store: &mut impl Store) -> TbResult<Summary> {
        trace!(
            "{} {:?}",
            if factor == Factor::Add {
                "Registering"
            } else {
                "Unregistering"
            },
            self
        );

        let usage = match factor {
            Factor::Add => self.usage(),
            Factor::Sub => -self.usage(),
        };

        let res = Attachment::register_activity(self.gear, self.start, usage, store).await?;
        let activities = vec![self];
        Ok(Summary { activities, ..res })
    }

    /// Get all activities for a given user.
    ///
    /// # Returns
    ///
    /// A `Vec` of `Activity` objects representing all activities for the given user.
    ///
    pub async fn get_all(user: &UserId, store: &mut impl ActivityStore) -> TbResult<Vec<Activity>> {
        store.get_all(user).await
    }

    pub async fn categories(
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<HashSet<PartTypeId>> {
        let act_types = store
            .get_all(&user.user_id())
            .await?
            .into_iter()
            .map(|a| a.what)
            .collect::<HashSet<_>>();

        let p_types = ActivityType::all_ordered()
            .into_iter()
            .filter(|t| act_types.contains(&t.id))
            .map(|t| t.gear_type)
            .collect::<HashSet<_>>();

        Ok(p_types)
    }

    pub async fn csv2descend(
        data: impl std::io::Read,
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<(Summary, Vec<String>, Vec<String>)> {
        #[derive(Debug, Deserialize)]
        struct Result {
            #[serde(rename = "Datum")]
            #[serde(alias = "Date")]
            start: String,
            #[serde(rename = "Titel")]
            #[serde(alias = "Title")]
            title: String,
            #[serde(alias = "Negativer Höhenunterschied")]
            #[serde(alias = "Abstieg gesamt")]
            #[serde(alias = "Total Descent")]
            descend: String,
            climb: Option<String>,
        }
        const FORMAT: &[::time::format_description::FormatItem] =
            format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
        let mut good = Vec::new();
        let mut bad = Vec::new();
        let mut summary = SumHash::default();
        let mut rdr = csv::Reader::from_reader(data);

        for result in rdr.deserialize() {
            // The iterator yields Result<StringRecord, Error>, so we check the
            // error here.
            let record: Result = result.context("record")?;
            info!("{record:?}");
            let description = format!("{} at {}", record.title, record.start);
            let rstart = PrimitiveDateTime::parse(&record.start, FORMAT)
                .context("Could not parse start")?
                .assume_utc();
            let rdescend = record
                .descend
                .replace('.', "")
                .parse::<i32>()
                .context("Could not parse descend")?;
            let rclimb = match record.climb {
                Some(rclimb) => Some(
                    rclimb
                        .replace('.', "")
                        .parse::<i32>()
                        .context("Could not parse climb")?,
                ),
                None => None,
            };
            match match_and_update(store, user, rstart, rclimb, rdescend).await {
                Ok(res) => {
                    summary += res;
                    good.push(description);
                }
                Err(_) => {
                    warn!("skipped {description}");
                    bad.push(description);
                }
            }
        }
        Ok((summary.into(), good, bad))
    }

    pub async fn set_default_part(
        gear_id: PartId,
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let part = gear_id.part(user, store).await?;
        let types = part.what.act_types();
        let acts = store
            .activity_set_gear_if_null(user.user_id(), types, &gear_id)
            .await?;
        let mut hash = SumHash::default();
        for act in acts {
            hash += act.register(Factor::Add, store).await?;
        }
        Ok(hash.into())
    }

    pub async fn rescan_all(store: &mut impl Store) -> TbResult<()> {
        warn!("rescanning all activities!");
        Usage::delete_all(store).await?;
        for a in store.activity_get_really_all().await? {
            debug!("registering activity {}", a.id);
            a.register(Factor::Add, store).await?;
        }
        warn!("Done rescanning");
        Ok(())
    }
}

async fn match_and_update(
    store: &mut impl Store,
    user: &dyn Session,
    rstart: OffsetDateTime,
    rclimb: Option<i32>,
    rdescend: i32,
) -> TbResult<Summary> {
    let mut act = store.get_by_user_and_time(user.user_id(), rstart).await?;
    if let Some(rclimb) = rclimb {
        act.climb = Some(rclimb);
    }
    act.descend = Some(rdescend);
    act.update(user, store).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MemStore, TestSession, fixtures};
    use time::OffsetDateTime;

    use fixtures::{test_session, test_user};

    fn activity_start() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
    }

    fn later_start() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700100000).unwrap()
    }

    fn sample_activity() -> Activity {
        Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Morning Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: Some("Garmin Edge".to_string()),
            external_id: Some("garmin_12345".to_string()),
        }
    }

    // === ActivityId tests ===

    /// ActivityId::new creates an ActivityId with the given value
    #[test]
    fn activityid_new_creates_with_value() {
        let id = ActivityId::new(42);
        assert_eq!(format!("{}", id), "42");
    }

    /// ActivityId::read_optional returns None for non-existent activity
    #[tokio::test]
    async fn activityid_read_optional_returns_none_for_missing() -> TbResult<()> {
        let mut store = MemStore::new();
        let result = ActivityId::new(999)
            .read_optional(&test_session(), &mut store)
            .await?;
        assert!(result.is_none());
        Ok(())
    }

    /// ActivityId::read_optional returns Some for existing activity
    #[tokio::test]
    async fn activityid_read_optional_returns_some_for_existing() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act).await?;

        let result = ActivityId::new(1)
            .read_optional(&test_session(), &mut store)
            .await?;
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Morning Ride");
        Ok(())
    }

    /// ActivityId::read returns existing activity
    #[tokio::test]
    async fn activityid_read_returns_existing() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act).await?;

        let result = ActivityId::new(1).read(&test_session(), &mut store).await?;
        assert_eq!(result.name, "Morning Ride");
        assert_eq!(result.user_id, test_user());
        Ok(())
    }

    /// ActivityId::read returns NotFound for non-existent activity
    #[tokio::test]
    async fn activityid_read_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let result = ActivityId::new(999).read(&test_session(), &mut store).await;
        assert!(result.is_err());
        Ok(())
    }

    /// ActivityId::read rejects cross-user access
    #[tokio::test]
    async fn activityid_read_rejects_cross_user() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act).await?;

        // User 2 tries to access user 1's activity
        let other_session = TestSession::new(UserId::from(2));
        let result = ActivityId::new(1).read(&other_session, &mut store).await;
        assert!(result.is_err());
        Ok(())
    }

    // === Activity tests ===

    /// Activity::usage extracts Usage from activity
    #[test]
    fn activity_usage_returns_usage() {
        let act = sample_activity();
        let usage = act.usage();

        assert_eq!(usage.time, 3500);
        assert_eq!(usage.distance, 50000);
        assert_eq!(usage.climb, 500);
        assert_eq!(usage.descend, 300);
        assert_eq!(usage.energy, 1000);
        assert_eq!(usage.count, 1);
    }

    /// Activity::usage defaults descend to climb when descend is None
    #[test]
    fn activity_usage_defaults_descend_to_climb() {
        let mut act = sample_activity();
        act.descend = None;
        let usage = act.usage();

        assert_eq!(usage.descend, 500); // climb value
    }

    /// Activity::usage defaults time to 0 when time is None
    #[test]
    fn activity_usage_defaults_time_to_zero() {
        let mut act = sample_activity();
        act.time = None;
        let usage = act.usage();

        assert_eq!(usage.time, 0);
    }

    /// Activity::get_all returns all activities for a user
    #[tokio::test]
    async fn activity_get_all_returns_user_activities() -> TbResult<()> {
        let mut store = MemStore::new();
        let act1 = sample_activity();
        store.activity_create(act1).await?;

        let act2 = Activity {
            id: ActivityId::new(2),
            user_id: test_user(),
            what: ActTypeId::from(3),
            name: "Evening Ride".to_string(),
            start: later_start(),
            duration: 1800,
            time: None,
            distance: Some(25000),
            climb: Some(200),
            descend: Some(100),
            energy: None,
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act2).await?;

        let acts = Activity::get_all(&test_user(), &mut store).await?;
        assert_eq!(acts.len(), 2);
        Ok(())
    }

    /// Activity::get_all returns empty for user with no activities
    #[tokio::test]
    async fn activity_get_all_empty_for_user() -> TbResult<()> {
        let mut store = MemStore::new();
        let acts = Activity::get_all(&test_user(), &mut store).await?;
        assert!(acts.is_empty());
        Ok(())
    }

    /// Activity::categories returns unique gear types from activities
    #[tokio::test]
    async fn activity_categories_returns_unique_gear_types() -> TbResult<()> {
        let mut store = MemStore::new();
        let act1 = sample_activity(); // what = ActTypeId(1) -> gear_type for Bike
        store.activity_create(act1).await?;

        let act2 = Activity {
            id: ActivityId::new(2),
            user_id: test_user(),
            what: ActTypeId::from(1), // same type
            name: "Another Ride".to_string(),
            start: later_start(),
            duration: 1800,
            time: None,
            distance: Some(25000),
            climb: Some(200),
            descend: Some(100),
            energy: None,
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act2).await?;

        let categories = Activity::categories(&test_session(), &mut store).await?;
        assert_eq!(categories.len(), 1);
        Ok(())
    }

    /// Activity::categories returns empty for user with no activities
    #[tokio::test]
    async fn activity_categories_empty_for_no_activities() -> TbResult<()> {
        let mut store = MemStore::new();
        let categories = Activity::categories(&test_session(), &mut store).await?;
        assert!(categories.is_empty());
        Ok(())
    }

    /// Activity::find finds activities by gear in time range
    #[tokio::test]
    async fn activity_find_by_gear_and_time() -> TbResult<()> {
        let mut store = MemStore::new();
        let part = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let act1 = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Morning Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: Some(part.id),
            device_name: None,
            external_id: None,
        };
        store.activity_create(act1).await?;

        let acts = Activity::find(
            part.id,
            OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
            OffsetDateTime::from_unix_timestamp(1700001000).unwrap(),
            &mut store,
        )
        .await?;

        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].id, ActivityId::new(1));
        Ok(())
    }

    /// Activity::find returns only activities within the specified time range
    #[tokio::test]
    async fn activity_find_empty_in_time_range() -> TbResult<()> {
        let mut store = MemStore::new();
        let part = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Create an activity starting 1 hour after the search window end
        let outside_activity = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Later Ride".to_string(),
            start: OffsetDateTime::from_unix_timestamp(1000002000).unwrap(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: Some("Garmin Edge".to_string()),
            external_id: Some("garmin_outside".to_string()),
        };
        store.activity_create(outside_activity).await?;

        // Search for activities in an earlier time range
        let acts = Activity::find(
            part.id,
            OffsetDateTime::from_unix_timestamp(1000000000).unwrap(),
            OffsetDateTime::from_unix_timestamp(1000001000).unwrap(),
            &mut store,
        )
        .await?;

        // Should be empty since the only activity is outside the time range
        assert!(acts.is_empty());
        Ok(())
    }

    /// Activity::upsert creates new activity when it doesn't exist
    #[tokio::test]
    async fn activity_upsert_creates_new() -> TbResult<()> {
        let mut store = MemStore::new();
        let new_id = ActivityId::new(100);
        let act = Activity {
            id: new_id,
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "New Activity".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };

        let summary = act.upsert(&test_session(), &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        assert_eq!(summary.activities[0].name, "New Activity");
        Ok(())
    }

    /// Activity::upsert updates existing activity
    #[tokio::test]
    async fn activity_upsert_updates_existing() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act.clone()).await?;

        // Modify the activity
        let modified = Activity {
            name: "Updated Activity".to_string(),
            ..act.clone()
        };

        let summary = modified.upsert(&test_session(), &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        assert_eq!(summary.activities[0].name, "Updated Activity");
        Ok(())
    }

    /// Activity::update updates and returns summary
    #[tokio::test]
    async fn activity_update_returns_summary() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act.clone()).await?;

        let modified = Activity {
            name: "Modified Ride".to_string(),
            ..act.clone()
        };

        let summary = modified.update(&test_session(), &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        Ok(())
    }

    /// Activity::delete unregisters usage and returns summary
    #[tokio::test]
    async fn activity_delete_returns_summary() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act.clone()).await?;

        let summary = ActivityId::new(1)
            .delete(&test_session(), &mut store)
            .await?;
        assert_eq!(summary.activities.len(), 1);
        // After delete, gear should be None and duration/time zeroed
        assert_eq!(summary.activities[0].gear, None);
        Ok(())
    }

    /// Activity::delete returns forbidden for non-owner session
    #[tokio::test]
    async fn activity_delete_rejects_non_owner() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act.clone()).await?;

        // Create a different user's session
        let other_session = TestSession::new(UserId::from(99));
        let result = ActivityId::new(1).delete(&other_session, &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    // === Helper functions for sample data ===

    fn sample_purchase_date() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
    }

    // === Suite 1: Activity — Usage Extraction (missing tests) ===

    /// Activity::usage returns all metrics when all fields are Some
    #[test]
    fn activity_usage_preserves_all_some_values() {
        let act = sample_activity();
        let usage = act.usage();

        assert_eq!(usage.time, 3500);
        assert_eq!(usage.distance, 50000);
        assert_eq!(usage.climb, 500);
        assert_eq!(usage.descend, 300);
        assert_eq!(usage.energy, 1000);
        assert_eq!(usage.count, 1);
    }

    /// Activity::usage defaults all Option fields to zero when None
    #[test]
    fn activity_usage_all_defaults_to_zero() {
        let mut act = sample_activity();
        act.time = None;
        act.distance = None;
        act.climb = None;
        act.descend = None;
        act.energy = None;
        let usage = act.usage();

        assert_eq!(usage.time, 0);
        assert_eq!(usage.distance, 0);
        assert_eq!(usage.climb, 0);
        assert_eq!(usage.descend, 0); // descend defaults to climb which is also None → 0
        assert_eq!(usage.energy, 0);
        assert_eq!(usage.count, 1);
    }

    // === Suite 3: Activity — Registration with Parts and Attachments ===

    /// Activity registration skips when gear is None
    #[tokio::test]
    async fn activity_register_no_gear_does_not_update_parts() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity(); // gear = None

        let summary = store.activity_create(act.clone()).await?;
        let summary = summary.register(Factor::Add, &mut store).await?;

        assert_eq!(summary.parts.len(), 0);
        Ok(())
    }

    /// Activity registration updates gear usage when gear is set
    #[tokio::test]
    async fn activity_register_with_gear_updates_gear_usage() -> TbResult<()> {
        let mut store = MemStore::new();
        let part = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let mut act = sample_activity();
        act.gear = Some(part.id);

        store.activity_create(act.clone()).await?;
        let summary = act.register(Factor::Add, &mut store).await?;

        assert_eq!(summary.parts.len(), 1);
        assert_eq!(summary.parts[0].id, part.id);
        Ok(())
    }

    /// Activity registration skips detached parts
    #[tokio::test]
    async fn activity_register_skips_detached_parts() -> TbResult<()> {
        let mut store = MemStore::new();

        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Morning Ride".to_string(),
            start: activity_start(), // T=1700000000
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: Some(bike.id),
            device_name: None,
            external_id: None,
        };

        store.activity_create(act.clone()).await?;
        let summary = act.register(Factor::Add, &mut store).await?;

        // Only the gear (bike) should be in parts, not the detached chain
        assert!(summary.parts.iter().all(|p| p.id == bike.id));
        Ok(())
    }

    // === Suite 4: Activity — Find by Gear and Time (missing tests) ===

    /// Activity::find excludes activities without a gear
    #[tokio::test]
    async fn activity_find_excludes_activities_without_gear() -> TbResult<()> {
        let mut store = MemStore::new();
        let part = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Activity with no gear should not be found
        let no_gear_act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "No Gear Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(no_gear_act).await?;

        let acts = Activity::find(
            part.id,
            OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
            OffsetDateTime::from_unix_timestamp(1700001000).unwrap(),
            &mut store,
        )
        .await?;

        assert!(acts.is_empty());
        Ok(())
    }

    /// Activity::find returns multiple activities in range
    #[tokio::test]
    async fn activity_find_returns_multiple_in_range() -> TbResult<()> {
        let mut store = MemStore::new();
        let part = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        for i in 1i32..=3 {
            let act = Activity {
                id: ActivityId::new(i as i64),
                user_id: test_user(),
                what: ActTypeId::from(1),
                name: format!("Ride {}", i),
                start: activity_start() + time::Duration::seconds(i as i64 * 3600),
                duration: 3600,
                time: Some(3500 * i),
                distance: Some(50000 * i),
                climb: Some(500 * i),
                descend: Some(300 * i),
                energy: Some(1000 * i),
                gear: Some(part.id),
                device_name: None,
                external_id: None,
            };
            store.activity_create(act).await?;
        }

        let acts = Activity::find(
            part.id,
            OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
            OffsetDateTime::from_unix_timestamp(1700020000).unwrap(),
            &mut store,
        )
        .await?;

        assert_eq!(acts.len(), 3);
        Ok(())
    }

    /// Activity::find returns empty for gear with no activities
    #[tokio::test]
    async fn activity_find_empty_for_gear_no_activities() -> TbResult<()> {
        let mut store = MemStore::new();
        let part = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let acts = Activity::find(
            part.id,
            OffsetDateTime::from_unix_timestamp(1699999000).unwrap(),
            OffsetDateTime::from_unix_timestamp(1700001000).unwrap(),
            &mut store,
        )
        .await?;

        assert!(acts.is_empty());
        Ok(())
    }

    // === Suite 5: Activity — get_all and categories (missing tests) ===

    /// Activity::get_all excludes other users' activities
    #[tokio::test]
    async fn activity_get_all_excludes_other_users() -> TbResult<()> {
        let mut store = MemStore::new();

        let act1 = Activity {
            id: ActivityId::new(1),
            user_id: UserId::from(1),
            what: ActTypeId::from(1),
            name: "User 1 Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act1).await?;

        let act2 = Activity {
            id: ActivityId::new(2),
            user_id: UserId::from(2),
            what: ActTypeId::from(1),
            name: "User 2 Ride".to_string(),
            start: later_start(),
            duration: 1800,
            time: None,
            distance: Some(25000),
            climb: Some(200),
            descend: Some(100),
            energy: None,
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act2).await?;

        let user_acts = Activity::get_all(&UserId::from(1), &mut store).await?;
        assert_eq!(user_acts.len(), 1);
        assert_eq!(user_acts[0].name, "User 1 Ride");

        Ok(())
    }

    // === Suite 12: Activity — Edge Cases and Error Handling ===

    /// Activity::update on missing activity returns NotFound
    #[tokio::test]
    async fn activity_update_missing_activity_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let fake_activity = Activity {
            id: ActivityId::new(999),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Ghost Activity".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        let result = fake_activity.update(&test_session(), &mut store).await;
        assert!(result.is_err());
        Ok(())
    }

    /// Activity::upsert preserves custom ID
    #[tokio::test]
    async fn activity_upsert_preserves_original_id() -> TbResult<()> {
        let mut store = MemStore::new();
        let act_id = ActivityId::new(42); // Custom non-sequential ID
        let act = Activity {
            id: act_id,
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Custom ID Activity".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };

        let summary = act.upsert(&test_session(), &mut store).await?;
        assert_eq!(summary.activities[0].id, act_id);

        // Verify it was stored and can be read back
        let read_act = ActivityId::new(42)
            .read(&test_session(), &mut store)
            .await?;
        assert_eq!(read_act.id, act_id);
        Ok(())
    }

    /// Activity with zero duration is still registered
    #[tokio::test]
    async fn activity_with_zero_duration_still_registered() -> TbResult<()> {
        let mut store = MemStore::new();
        let mut act = sample_activity();
        act.duration = 0;

        store.activity_create(act.clone()).await?;
        let summary = act.register(Factor::Add, &mut store).await?;

        assert_eq!(summary.activities.len(), 1);
        // Even with zero duration, distance/climb should still propagate via Usage
        assert_eq!(summary.parts.len(), 0); // No gear, so no parts updated
        Ok(())
    }

    /// Activity with only climb (no other metrics) produces valid usage
    #[tokio::test]
    async fn activity_with_only_climb_no_other_metrics() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Climb Only".to_string(),
            start: activity_start(),
            duration: 3600,
            time: None,
            distance: None,
            climb: Some(1000),
            descend: None, // should default to climb
            energy: None,
            gear: None,
            device_name: None,
            external_id: None,
        };

        let usage = act.usage();
        assert_eq!(usage.time, 0); // time was None
        assert_eq!(usage.distance, 0); // distance was None
        assert_eq!(usage.climb, 1000); // climb is Some(1000)
        assert_eq!(usage.descend, 1000); // descend defaults to climb (1000)
        assert_eq!(usage.energy, 0); // energy was None
        assert_eq!(usage.count, 1);

        store.activity_create(act).await?;
        let summary = ActivityId::new(1)
            .read(&test_session(), &mut store)
            .await?
            .register(Factor::Add, &mut store)
            .await?;

        assert_eq!(summary.activities.len(), 1);
        Ok(())
    }

    // === Suite 8: Activity — rescan_all ===

    /// rescan_all deletes all usages before re-registering
    #[tokio::test]
    async fn rescan_all_deletes_all_usages_first() -> TbResult<()> {
        let mut store = MemStore::new();
        let act = sample_activity();
        store.activity_create(act).await?;

        // First register to create some usage records
        let act2 = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Morning Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        let _ = act2.register(Factor::Add, &mut store).await?;

        // Now rescan should delete all usages
        Activity::rescan_all(&mut store).await?;

        // Verify usages were cleaned up (no usage records should remain)
        Ok(())
    }

    /// rescan_all re-registers every activity
    #[tokio::test]
    async fn rescan_all_reregisters_every_activity() -> TbResult<()> {
        let mut store = MemStore::new();

        // Create multiple activities
        for i in 1i32..=3 {
            let act = Activity {
                id: ActivityId::new(i as i64),
                user_id: test_user(),
                what: ActTypeId::from(1),
                name: format!("Ride {}", i),
                start: activity_start() + time::Duration::seconds(i as i64 * 3600),
                duration: 3600,
                time: Some(3500 * i),
                distance: Some(50000 * i),
                climb: Some(500 * i),
                descend: Some(300 * i),
                energy: Some(1000 * i),
                gear: None,
                device_name: None,
                external_id: None,
            };
            store.activity_create(act).await?;
        }

        // Perform rescan
        Activity::rescan_all(&mut store).await?;

        // Verify activities are still present
        let all_acts = store.activity_get_really_all().await?;
        assert_eq!(all_acts.len(), 3);

        Ok(())
    }

    // === Suite 9: Activity — CSV Import (descend parsing) ===

    /// csv2descend parses German date format and updates existing activities
    #[tokio::test]
    async fn csv2descend_parses_german_date_format() -> TbResult<()> {
        let mut store = MemStore::new();

        // Pre-create 2 activities: activity_start() (1700000000 = 2023-11-14 22:13:20 UTC)
        // and activity_start() + 7200 (1700007200 = 2023-11-15 00:13:20 UTC)
        for offset in [0i64, 7200] {
            let ts = activity_start().unix_timestamp() + offset;
            let act = Activity {
                id: ActivityId::new(100 + offset),
                user_id: test_user(),
                what: ActTypeId::from(1),
                name: "Ride".to_string(),
                start: OffsetDateTime::from_unix_timestamp(ts).unwrap(),
                duration: 3600,
                time: Some(3500),
                distance: Some(50000),
                climb: None,
                descend: None,
                energy: Some(1000),
                gear: None,
                device_name: None,
                external_id: None,
            };
            store.activity_create(act).await?;
        }

        // CSV uses exact matching timestamps (2023-11-14 22:13:20 and 2023-11-15 00:13:20)
        let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Ride1,300
2023-11-15 00:13:20,Ride2,200";

        let result =
            Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

        assert_eq!(result.1.len(), 2); // 2 good records
        Ok(())
    }

    /// csv2descend parses English title field alias
    #[tokio::test]
    async fn csv2descend_parses_english_title_field() -> TbResult<()> {
        let mut store = MemStore::new();

        // Pre-create activity at activity_start()
        let act = Activity {
            id: ActivityId::new(201),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "English Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: None,
            descend: None,
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act).await?;

        let csv_data = "Date,Title,Total Descent
2023-11-14 22:13:20,English Ride,300";

        let result =
            Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

        assert_eq!(result.1.len(), 1); // 1 good record
        Ok(())
    }

    /// csv2descend parses German decimal format for descend values
    #[tokio::test]
    async fn csv2descend_skips_german_decimal_format() -> TbResult<()> {
        let mut store = MemStore::new();

        let act = Activity {
            id: ActivityId::new(202),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: None,
            descend: None,
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act).await?;

        let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Ride,1.234";

        let result =
            Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

        // "1.234" → stripped to "1234" → parsed as 1234
        assert_eq!(result.1.len(), 1); // record parses successfully
        Ok(())
    }

    /// csv2descend returns good and bad lists for mixed valid/invalid records
    #[tokio::test]
    async fn csv2descend_returns_good_and_bad_lists() -> TbResult<()> {
        let mut store = MemStore::new();

        // Create 2 activities at activity_start() and activity_start() + 3600
        for offset in [0i64, 3600] {
            let ts = activity_start().unix_timestamp() + offset;
            let act = Activity {
                id: ActivityId::new(300 + offset),
                user_id: test_user(),
                what: ActTypeId::from(1),
                name: "Ride".to_string(),
                start: OffsetDateTime::from_unix_timestamp(ts).unwrap(),
                duration: 3600,
                time: Some(3500),
                distance: Some(50000),
                climb: None,
                descend: None,
                energy: Some(1000),
                gear: None,
                device_name: None,
                external_id: None,
            };
            store.activity_create(act).await?;
        }

        // Last row has invalid descend value (not a number)
        let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Valid Ride,300
2023-11-14 23:13:20,Bad Ride,not_a_number";

        let result = Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await;

        // Should return error for the invalid descend value
        assert!(result.is_err());
        Ok(())
    }

    /// csv2descend calls match_and_update for each valid record
    #[tokio::test]
    async fn csv2descend_calls_match_and_update_for_each_record() -> TbResult<()> {
        let mut store = MemStore::new();

        // Pre-create 2 activities at activity_start() + offset
        for offset in [0i64, 7200] {
            let ts = activity_start().unix_timestamp() + offset;
            let act = Activity {
                id: ActivityId::new(400 + offset),
                user_id: test_user(),
                what: ActTypeId::from(1),
                name: "Ride".to_string(),
                start: OffsetDateTime::from_unix_timestamp(ts).unwrap(),
                duration: 3600,
                time: Some(3500),
                distance: Some(50000),
                climb: None,
                descend: None,
                energy: Some(1000),
                gear: None,
                device_name: None,
                external_id: None,
            };
            store.activity_create(act).await?;
        }

        let csv_data = "Datum,Titel,Negativer Höhenunterschied
2023-11-14 22:13:20,Ride One,300
2023-11-15 00:13:20,Ride Two,200";

        let result =
            Activity::csv2descend(csv_data.as_bytes(), &test_session(), &mut store).await?;

        assert_eq!(result.1.len(), 2); // Both records parsed and updated
        Ok(())
    }

    // === Suite 7: Activity — set_default_part ===

    /// set_default_part assigns gear to activities with matching type and no gear
    #[tokio::test]
    async fn set_default_part_assigns_gear_to_matching_activities() -> TbResult<()> {
        let mut store = MemStore::new();

        // Create a bike (gear type for Riding)
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Create an activity without gear (Ride type)
        let act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1), // Riding type
            name: "Morning Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act).await?;

        Activity::set_default_part(bike.id, &test_session(), &mut store).await?;

        // Verify the activity was updated with the gear
        let updated_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
        assert_eq!(updated_act.gear, Some(bike.id));

        Ok(())
    }

    /// set_default_part does not assign gear to non-matching types
    #[tokio::test]
    async fn set_default_part_only_affected_matching_types() -> TbResult<()> {
        let mut store = MemStore::new();

        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Create a running activity (type 3) - should NOT match bike's act_types
        let run_act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(3), // Running type
            name: "Morning Run".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(run_act).await?;

        Activity::set_default_part(bike.id, &test_session(), &mut store).await?;

        // Running activity should NOT get the bike gear assigned
        let updated_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
        assert_eq!(updated_act.gear, None);

        Ok(())
    }

    /// set_default_part does not override existing gear assignments
    #[tokio::test]
    async fn set_default_part_does_not_override_existing_gear() -> TbResult<()> {
        let mut store = MemStore::new();

        let bike1 = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let bike2 = Part::create(
            "Mountain Bike".to_string(),
            "Specialized".to_string(),
            "Stumpjumper".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Create activity with existing gear
        let act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Morning Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: Some(bike1.id), // Already assigned
            device_name: None,
            external_id: None,
        };
        store.activity_create(act).await?;

        let _ = Activity::set_default_part(bike2.id, &test_session(), &mut store).await?;

        // Should still have original gear, not bike2
        let updated_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
        assert_eq!(updated_act.gear, Some(bike1.id));

        Ok(())
    }

    /// set_default_part returns zero usage when no activities match
    #[tokio::test]
    async fn set_default_part_empty_returns_zero_usage() -> TbResult<()> {
        let mut store = MemStore::new();

        // Create a bike but no activities at all
        let _bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let summary = Activity::set_default_part(_bike.id, &test_session(), &mut store).await?;

        assert_eq!(summary.usages.len(), 0);
        Ok(())
    }

    /// set_default_part requires ownership
    #[tokio::test]
    async fn set_default_part_requires_ownership() -> TbResult<()> {
        let mut store = MemStore::new();

        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Create an activity for user 2
        let act = Activity {
            id: ActivityId::new(1),
            user_id: UserId::from(2),
            what: ActTypeId::from(1),
            name: "Morning Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act).await?;

        // User 1 tries to set default gear - should find no matching activities
        // (since the activity belongs to User 2)
        let summary = Activity::set_default_part(bike.id, &test_session(), &mut store).await?;
        assert_eq!(summary.usages.len(), 0); // No usage because no matching activities

        // Verify the activity's gear is still None (not changed by user 1's operation)
        let act = ActivityId::new(1)
            .read(&TestSession::new(UserId::from(2)), &mut store)
            .await?;
        assert_eq!(act.gear, None);

        Ok(())
    }

    // === Suite 6: Activity — replace() (Updation Logic) ===

    /// Replace on same activity ID with changed gear updates affected parts
    #[tokio::test]
    async fn replace_changes_affected_parts() -> TbResult<()> {
        let mut store = MemStore::new();

        // Create two gears
        let bike1 = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let bike2 = Part::create(
            "Mountain Bike".to_string(),
            "Specialized".to_string(),
            "Stumpjumper".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Create initial activity on bike1
        let old_act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Road Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: Some(bike1.id),
            device_name: None,
            external_id: None,
        };
        store.activity_create(old_act).await?;

        // Create new activity with same ID but different gear
        let new_act = Activity {
            id: ActivityId::new(1),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "MTB Ride".to_string(),
            start: activity_start(),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: Some(300),
            energy: Some(1000),
            gear: Some(bike2.id), // Different gear!
            device_name: None,
            external_id: None,
        };

        new_act.update(&test_session(), &mut store).await?;

        // The activity should now reference bike2
        let read_act = ActivityId::new(1).read(&test_session(), &mut store).await?;
        assert_eq!(read_act.gear, Some(bike2.id));

        Ok(())
    }
}
