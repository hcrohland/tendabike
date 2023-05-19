//! Activity handling for the TendaBike backend
//!
//! struct Activity captures all data of an athlete's activity
//!
//! By assigning a gear to the activity it gets accounted with that gear and all it's parts attached
//! at the start time of the activity
//! Most operations are done on the ActivityId though
//!

use super::*;
use schema::activities;

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
#[diesel(table_name = activities)]
pub struct Activity {
    /// The primary key
    pub id: ActivityId,
    /// The athlete
    pub user_id: i32,
    /// The activity type
    pub what: ActTypeId,
    /// This name of the activity.
    pub name: String,
    /// Start time
    pub start: DateTime<Utc>,
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

#[derive(Debug, Clone, Insertable, AsChangeset, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = activities)]
pub struct NewActivity {
    pub user_id: i32,
    /// The activity type
    pub what: ActTypeId,
    /// This name of the activity.
    pub name: String,
    /// Start time
    pub start: DateTime<Utc>,
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

impl ActivityId {
    pub fn new(id:i32) -> Self {
        Self(id)
    }

    /// Read the activity with id self
    ///
    /// checks authorization
    pub fn read(self, person: &dyn Person, conn: &mut AppConn) -> AnyResult<Activity> {
        let act = activities::table
            .find(self)
            .for_update()
            .first::<Activity>(conn)
            .context(format!("No activity id {}", self))?;
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
    pub fn delete(self, person: &dyn Person, conn: &mut AppConn) -> AnyResult<Summary> {
        use schema::activities::dsl::*;
        info!("Deleting {:?}", self);
        conn.transaction(|conn| {
            let mut res = self
                .read(person, conn)
                .context("Could not read user")?
                .register(Factor::Sub, conn)
                .context("could not unregister activity")?;
            diesel::delete(activities.filter(id.eq(self)))
                .execute(conn)
                .context("Error deleting activity")?;
            res.activities[0].gear=None;
            res.activities[0].duration=0;
            res.activities[0].time=None;
            res.activities[0].distance=None;
            res.activities[0].climb=None;
            res.activities[0].descend=None;
            res.activities[0].power=None;
            Ok(res)
        })
    }

    /// Update the activity with id self according to the data in NewActivity
    /// and update part usage accordingly
    ///
    /// returns all affected parts  
    /// checks authorization  
    pub fn update(
        self,
        act: &NewActivity,
        user: &dyn Person,
        conn: &mut AppConn,
    ) -> AnyResult<Summary> {
        conn.transaction(|conn| {
            self
                .read(user, conn)?
                .register(Factor::Sub, conn)?;
                
            let act = diesel::update(activities::table)
                .filter(activities::id.eq(self))
                .set(act)
                .get_result::<Activity>(conn)
                .context("Error updating activity")?;

            info!("Updating {:?}", act);

            let res = act.register(Factor::Add, conn)
                            .context("Could not register activity")?;
            Ok(res)
        })
    }
}

impl Activity {
    /// create a new activity
    ///
    /// returns the activity and all affected parts  
    /// checks authorization  
    pub fn create(
        act: &NewActivity,
        user: &dyn Person,
        conn: &mut AppConn,
    ) -> AnyResult<Summary> {
        user.check_owner(
            act.user_id,
            format!(
                "user {} cannot create activity for user {}",
                user.get_id(),
                act.user_id
            ),
        )?;
        info!("Creating {:?}", act);
        conn.transaction(|conn| {
            let new: Activity = diesel::insert_into(activities::table)
                .values(act)
                .get_result(conn)
                .context("Could not insert activity")?;
            // let res = new.check_geartype(res, conn)?;
            new
                .register(Factor::Add, conn)
                .context("Could not register activity")
        })
    }

    /// Extract the usage out of an activity
    ///
    /// If the descend value is missing, assume descend = climb
    /// Account for Factor
    pub fn usage(&self, factor: Factor) -> Usage {
        let factor = factor as i32;
        Usage {
            time: self.time.unwrap_or(0) * factor,
            distance: self.distance.unwrap_or(0) * factor,
            climb: self.climb.unwrap_or(0) * factor,
            descend: self.descend.unwrap_or_else(|| self.climb.unwrap_or(0)) * factor,
            power: self.power.unwrap_or(0) * factor,
            count: factor,
        }
    }

    /// find all activities for gear part in the given time frame
    ///
    /// if end is none it means for the whole future
    pub fn find(
        part: PartId,
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
        conn: &mut AppConn,
    ) -> Vec<Activity> {
        use schema::activities::dsl::{activities, gear, start};

        activities
            .filter(gear.eq(Some(part)))
            .filter(start.ge(begin))
            .filter(start.lt(end))
            .load::<Activity>(conn)
            .expect("could not read activities")
    }

    pub fn register(self, factor: Factor, conn: &mut AppConn) -> AnyResult<Summary> {
        trace!("{} {:?}", if factor == Factor::Add {"Registering"} else {"Unregistering"}, self);

        let usage = self.usage(factor);
        Ok(
            Summary {
                parts: Attachment::parts_per_activity(&self, conn)
                    .iter()
                    .map(|part| part.apply_usage(&usage, self.start, conn))
                    .collect::<AnyResult<_>>()?,
                attachments: Attachment::register(&self, &usage, conn),
                activities: vec![self]
            }
        )
    }

    pub fn get_all(user: &dyn Person, conn: &mut AppConn) -> AnyResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        let acts = activities
            .filter(user_id.eq(user.get_id()))
            .get_results::<Activity>(conn)
            .context(format!("Error reading activities for user {}", user.get_id()))?; 
        Ok(acts)
    }

    pub fn categories(user: &dyn Person, conn: &mut AppConn) -> AnyResult<Vec<PartTypeId>> {
        use crate::schema::activities::dsl::*;
        use crate::schema::activity_types;

        let act_types = activities
            .filter(user_id.eq(user.get_id()))
            .select(what)
            .distinct()
            .get_results::<ActTypeId>(conn)?;

        let p_types = activity_types::table
            .filter(activity_types::id.eq_any(act_types))
            .filter(activity_types::id.ne(0)) // catch-all unsupported
            .select(activity_types::gear)
            .distinct()
            .get_results(conn)?;

        Ok(p_types)

    }


    pub fn csv2descend(data: impl std::io::Read, tz: String, user: &User, conn: &mut AppConn) 
        -> AnyResult<(Summary, Vec<String>, Vec<String>)> {
        use schema::activities::dsl::*;
        #[derive(Debug, Deserialize)]
        struct Result {
            #[serde(rename = "Datum")]
            start: String,
            #[serde(rename = "Titel")]
            title: String,
            #[serde(alias = "Negativer Höhenunterschied")]
            #[serde(alias = "Abstieg gesamt")]
            #[serde(alias = "Total Descent")]
            descend: String,
            climb: Option<String>,
        }

        let mut good = Vec::new();
        let mut bad = Vec::new();
        let mut summary = Summary::default();
        let mut rdr = csv::Reader::from_reader(data);
        let tz = tz.parse::<chrono_tz::Tz>()
                    .map_err(|_| Error::BadRequest(format!("Unknown timezone {}",tz)))?;

        for result in rdr.deserialize() {
            // The iterator yields Result<StringRecord, Error>, so we check the
            // error here.
            let record: Result = result.context("record")?;
            info!("{:?}", record);
            let description = format!("{} at {}", &record.title, &record.start);
            let rstart = tz.datetime_from_str(&record.start, "%Y-%m-%d %H:%M:%S")?;
            let rdescend = record.descend.replace('.', "").parse::<i32>().context("Could not parse descend")?;
            let rclimb = match record.climb {
                Some(rclimb) => Some(rclimb.replace('.', "").parse::<i32>().context("Could not parse climb")?),
                None => None,
            };
            match 
                conn.transaction::<_,anyhow::Error,_>(|conn| {
                    let act: Activity = activities
                        .filter(user_id.eq(user.get_id()))
                        .filter(start.eq(rstart))
                        .for_update()
                        .get_result(conn).context(format!("Activitiy {}", rstart))?;
                    let act_id = act.register(Factor::Sub, conn)?
                        .activities[0].id;
                    if let Some(rclimb) = rclimb {
                        diesel::update(activities.find(act_id))
                            .set(climb.eq(rclimb))
                        .execute(conn)
                        .context("Error updating climb")?;    
                    }
                    let act = diesel::update(activities.find(act_id))
                        .set(descend.eq(rdescend))
                        .get_result::<Activity>(conn)
                        .context("Error updating descent")?;
                    act.register(Factor::Add, conn)
                        .context("Could not register activity")
                    }) 
                {
                    Ok(res) => {
                        summary = summary.merge(res);
                        good.push(description);
                    },
                    Err(_) => {
                        warn!("skipped {}", description); 
                        bad.push(description);
                    }
                }
        }   

        Ok((summary, good, bad))
    }

    pub fn set_default_part (gear_id: PartId, user: &User, conn: &mut AppConn) -> AnyResult<Summary>{
        conn.transaction(|conn| {
            def_part(&gear_id, user, conn)
        })
    }
    pub fn rescan_all (conn: &mut AppConn) -> AnyResult<()>{
        warn!("rescanning all activities!");
        let res = conn.transaction(|conn| {
            {
                use schema::parts::dsl::*;
                debug!("resetting all parts");
                diesel::update(parts).set((
                    time.eq(0),
                    distance.eq(0),
                    climb.eq(0),
                    descend.eq(0),
                    count.eq(0),
                )).execute(conn)?;
            }
            {
                use schema::attachments::dsl::*;
                debug!("resetting all attachments");
                diesel::update(attachments).set((
                    time.eq(0),
                    distance.eq(0),
                    climb.eq(0),
                    descend.eq(0),
                    count.eq(0),
                )).execute(conn)?;
            }
            {
                use schema::activities::dsl::*;
                for a in activities.order_by(id).get_results::<Activity>(conn)? {
                    debug!("registering activity {}", a.id);
                    a.register(Factor::Add, conn)?;
                }
            }
            Ok(())
        });
        warn!("Done rescanning");
        res
    }
    }

fn def_part(partid: &PartId, user: & User, conn: &mut AppConn) -> AnyResult<Summary> {
    use schema::activities::dsl::*;
    let part = partid.part(user, conn)?;
    let types = part.what.act_types(conn)?;

    let acts =
    diesel::update(activities)
        .filter(user_id.eq(user.get_id()))
        .filter(gear.is_null())
        .filter(what.eq_any(types))
        .set(gear.eq(partid))
        .get_results::<Activity>(conn)
        .context("Error updating activities")?;

    let mut hash = SumHash::default();
    for act in acts.into_iter() {
        hash.merge(act.register(Factor::Add, conn)?)
    }
    Ok(hash.collect())
}

