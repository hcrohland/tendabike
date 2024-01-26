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
use diesel_derive_newtype::*;
use newtype_derive::*;
use scoped_futures::ScopedFutureExt;
use serde_derive::{Deserialize, Serialize};
use time::{macros::format_description, OffsetDateTime, PrimitiveDateTime};
use time_tz::PrimitiveDateTimeExt;

use crate::*;

/// The Id of an Activity
///
/// Most operations for activities are done on the Id alone
///
#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityId(i32);

NewtypeDisplay! { () pub struct ActivityId(); }
NewtypeFrom! { () pub struct ActivityId(i32); }

/// The database's representation of an activity.
#[derive(Debug, Clone, Identifiable, Queryable, AsChangeset, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = schema::activities)]
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
    /// average power output
    pub power: Option<i32>,
    /// Which gear did she use?
    pub gear: Option<PartId>,
}

#[derive(Debug, Clone, Insertable, AsChangeset, Queryable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = schema::activities)]
/// A new activity to be inserted into the database.
pub struct NewActivity {
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
    /// average power output
    pub power: Option<i32>,
    /// Which gear did she use?
    pub gear: Option<PartId>,
}

impl From<Activity> for NewActivity {
    fn from(act: Activity) -> Self {
        Self {
            user_id: act.user_id,
            what: act.what,
            name: act.name,
            start: act.start,
            duration: act.duration,
            time: act.time,
            distance: act.distance,
            climb: act.climb,
            descend: act.descend,
            power: act.power,
            gear: act.gear,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Factor {
    Add = 1,
    Sub = -1,
}

impl ActivityId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    /// Read the activity with id self
    ///
    /// checks authorization
    pub async fn read(
        self,
        person: &dyn Person,
        store: &mut impl ActivityStore,
    ) -> TbResult<Activity> {
        let act = store.activity_read_by_id(self).await?;
        person.check_owner(
            act.user_id,
            format!("User {} cannot access activity {}", person.get_id(), self),
        )?;
        Ok(act)
    }

    /// Delete the activity with id self
    /// and update part usage accordingly
    ///
    /// returns all affected parts  
    /// checks authorization  
    pub async fn delete(self, person: &dyn Person, store: &mut impl Store) -> TbResult<Summary> {
        info!("Deleting {:?}", self);
        store
            .transaction(|store| {
                async {
                    let mut res = self
                        .read(person, store)
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
                    res.activities[0].power = None;
                    Ok(res)
                }
                .scope_boxed()
            })
            .await
    }

    /// Update the activity with id self according to the data in NewActivity
    /// and update part usage accordingly
    ///
    /// returns all affected parts  
    /// checks authorization  
    pub async fn update(
        self,
        act: &NewActivity,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        store
            .transaction(|store| {
                async {
                    self.read(user, store)
                        .await?
                        .register(Factor::Sub, store)
                        .await?;

                    let act = store.activity_update(self, act).await?;

                    info!("Updating {:?}", act);

                    let res = act.register(Factor::Add, store).await?;
                    Ok(res)
                }
                .scope_boxed()
            })
            .await
    }
}

impl Activity {
    /// create a new activity
    ///
    /// returns the activity and all affected parts  
    /// checks authorization  
    pub async fn create(
        act: &NewActivity,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        user.check_owner(
            act.user_id,
            format!(
                "user {} cannot create activity for user {}",
                user.get_id(),
                act.user_id
            ),
        )?;
        info!("Creating {:?}", act);
        store
            .transaction(|store| {
                async {
                    let new = store.activity_create(act).await?;
                    // let res = new.check_geartype(res, store)?;
                    new.register(Factor::Add, store).await
                }
                .scope_boxed()
            })
            .await
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
            power: self.power.unwrap_or(0),
            count: 1,
        }
    }

    /// find all activities for gear part in the given time frame
    ///
    /// if end is none it means for the whole future
    pub(crate) async fn find(
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
        store: &mut impl ActivityStore,
    ) -> TbResult<Vec<Activity>> {
        store
            .activities_find_by_partid_and_time(part, begin, end)
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

        let mut res = Attachment::register_activity(self.gear, self.start, usage, store).await?;
        res.activities = vec![self];
        Ok(res)
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
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<HashSet<PartTypeId>> {
        let act_types = store
            .get_all(&user.get_id())
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
        tz: String,
        user: &dyn Person,
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
        let tz = time_tz::timezones::get_by_name(&tz)
            .ok_or_else(|| Error::BadRequest(format!("Unknown timezone {}", tz)))?;

        for result in rdr.deserialize() {
            // The iterator yields Result<StringRecord, Error>, so we check the
            // error here.
            let record: Result = result.context("record")?;
            info!("{:?}", record);
            let description = format!("{} at {}", &record.title, &record.start);
            let rstart = PrimitiveDateTime::parse(&record.start, FORMAT)
                .context("Could not parse start")?
                .assume_timezone(tz)
                .unwrap();
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
            match store
                .transaction(|store| {
                    match_and_update(store, user, rstart, rclimb, rdescend).scope_boxed()
                })
                .await
            {
                Ok(res) => {
                    summary.merge(res);
                    good.push(description);
                }
                Err(_) => {
                    warn!("skipped {}", description);
                    bad.push(description);
                }
            }
        }
        let summary = summary.collect();
        Ok((summary, good, bad))
    }

    pub async fn set_default_part(
        gear_id: PartId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        store
            .transaction(|store| def_part(&gear_id, user, store).scope_boxed())
            .await
    }

    pub async fn rescan_all(store: &mut impl Store) -> TbResult<()> {
        warn!("rescanning all activities!");
        store
            .transaction(|store| rescan(store).scope_boxed())
            .await?;
        warn!("Done rescanning");
        Ok(())
    }
}

async fn rescan(store: &mut impl Store) -> TbResult<()> {
    Usage::delete_all(store).await?;
    for a in store.activity_get_really_all().await? {
        debug!("registering activity {}", a.id);
        a.register(Factor::Add, store).await?;
    }
    Ok(())
}

async fn match_and_update(
    store: &mut impl Store,
    user: &dyn Person,
    rstart: OffsetDateTime,
    rclimb: Option<i32>,
    rdescend: i32,
) -> TbResult<Summary> {
    let mut act = store.get_by_user_and_time(user.get_id(), rstart).await?;
    if let Some(rclimb) = rclimb {
        act.climb = Some(rclimb);
    }
    act.descend = Some(rdescend);
    let actid = act.id;
    let act = NewActivity::from(act);
    actid.update(&act, user, store).await
}

async fn def_part(partid: &PartId, user: &dyn Person, store: &mut impl Store) -> TbResult<Summary> {
    let part = partid.part(user, store).await?;
    let types = part.what.act_types();

    let acts = store.activity_set_gear_if_null(user, types, partid).await?;

    let mut hash = SumHash::default();
    for act in acts {
        hash.merge(act.register(Factor::Add, store).await?)
    }
    Ok(hash.collect())
}
