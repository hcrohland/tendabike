use chrono::{DateTime, Utc};

use super::*;
use crate::activity::ActivityId;
use crate::activity::NewActivity;
use strava::auth::User;

#[derive(Serialize, Deserialize, Debug)]
pub struct StravaActivity {
    pub id: i64,
    /// The activity type
    #[serde(rename = "type")]
    pub type_: String,
    /// This name of the activity.
    pub name: String,
    /// Start time
    pub start_date: DateTime<Utc>,
    /// End time
    pub elapsed_time: i32,
    /// activity time
    pub moving_time: i32,
    /// Covered distance
    pub distance: f64,
    /// Total climbing
    pub total_elevation_gain: f64,
    /// average power output
    pub average_watts: Option<f64>,
    /// Which gear did she use?
    pub gear_id: Option<String>,
}

impl StravaActivity {
    fn into_tb(self, user: &User) -> TbResult<NewActivity> {
        let what = self.what()?;
        let gear = match self.gear_id {
            Some(x) => Some(gear::strava_to_tb(x, user)?),
            None => None,
        };
        Ok(NewActivity {
            what,
            gear,
            user_id: user.tb_id(),
            name: self.name,
            start: self.start_date,
            duration: self.elapsed_time,
            time: Some(self.moving_time),
            distance: Some(self.distance.round() as i32),
            climb: Some(self.total_elevation_gain.round() as i32),
            descend: None,
            power: self.average_watts.map(|p| p.round() as i32),
        })
    }

    /// map strava workout type strings to tendabike types
    fn what(&self) -> TbResult<ActTypeId> {
        let t = self.type_.as_str();

        Ok(match t {
            "Ride"          => 1,
            "VirtualRide"   => 5,
            "EBikeRide"     => 9,
            "Snowboard"     => 2,
            "Run"           => 3,
            "VirtualRun"    => 3,
            "Hike"          => 4,
            "AlpineSki"     => 6,
            "Walk"          => 8,
            "BackcountrySki" => 10,
            "Workout"       => 0,
            "StandUpPaddling" => 0,
            "Windsurf"      => 0,
            "Kitesurf"      => 0,
            "Rowing"        => 0,
            "WaterSport"    => 0,
            "RockClimbing"  => 0,
            "Handcycle" => 0,
            "Canoeing" => 0,
            "Crossfit" => 0,
            "Elliptical" => 0,
            "Golf" => 0,
            "IceSkate" => 0,
            "InlineSkate" => 0,
            "Kayaking" => 0,
            "NordicSki" => 0,
            "RollerSki" => 0,
            "Sail" => 0,
            "Skateboard" => 0,
            "Snowshoe" => 0,
            "Soccer" => 0,
            "StairStepper" => 0,
            "Surfing" => 0,
            "Swim" => 0,
            "Velomobile" => 0,
            "WeightTraining" => 0,
            "Wheelchair" => 0,
            "Yoga" => 0,
            _ => bail!("unsupported activity {}", t)
        }.into())
    }
}

impl StravaActivity {
    fn send_to_tb(self, user: &User) -> TbResult<Summary> {
        user.conn().transaction(||{
            use schema::strava_activities::dsl::*;

            let strava_id = self.id;
            let tb = self.into_tb(user)?;

            let tb_id = strava_activities
                .find(strava_id)
                .select(tendabike_id)
                .for_update()
                .get_result::<ActivityId>(user.conn())
                .optional()?;

            let res; 
            if let Some(tb_id) = tb_id {
                res = tb_id.update(&tb, user, user.conn())?
            } else {
                res = Activity::create(&tb, user, user.conn())?;
                let new_id = &res.activities[0].id;
                diesel::insert_into(strava_activities)
                    .values((
                        id.eq(strava_id),
                        tendabike_id.eq(new_id),
                        user_id.eq(tb.user_id),
                    ))
                    .execute(user.conn())?;
            }

            user.update_last(tb.start.timestamp())
                .context("unable to update user")?;

            Ok(res)
        })
    }
}

pub fn strava_url(act: i32, user: &User) -> TbResult<String> {
    use schema::strava_activities::dsl::*;

    let g: i64 = strava_activities
        .filter(tendabike_id.eq(act))
        .select(id)
        .first(user.conn())?;

    Ok(format!("https://strava.com/activities/{}", &g))
}

fn get_activity(id: i64, user: &User) -> TbResult<StravaActivity> {
    let r = user.request(&format!("/activities/{}",id ))?;
    // let r = user.request("/activities?per_page=2")?;
    let act: StravaActivity = serde_json::from_str(&r)?;
    Ok(act)
}

fn upsert_activity(id: i64, user: &User) -> TbResult<Summary> {
    let act = get_activity(id, user).context(format!("strava activity id {}", id))?;
    act.send_to_tb(user)
}

fn delete_activity(sid: i64, user: &User) -> TbResult<Summary> {
    use schema::strava_activities::dsl::*;

    user.conn().transaction(||{
        let tid: Option<ActivityId> = strava_activities.select(tendabike_id).find(sid).for_update().first(user.conn()).optional()?;
        if let Some(tid) = tid {
            diesel::delete(strava_activities.find(sid)).execute(user.conn())?;
            tid.delete(user, user.conn())
        } else {
            Ok(Summary::default())
        }
    })
}

pub fn process_hook(e: &webhook::Event, user: &User) -> TbResult<Summary>{
    let res = match e.aspect_type.as_str() {
        "create" | "update" => upsert_activity(e.object_id, user)?,
        "delete" => delete_activity(e.object_id, user)?,
        _ => {
            warn!("Skipping unknown aspect_type {:?}", e);
            Summary::default()
        }
    };
    e.delete(user.conn())?;
    Ok(res)
}

fn next_activities(user: &User, per_page: usize, start: Option<i64>) -> TbResult<Vec<StravaActivity>> {
    let r = user.request(&format!(
        "/activities?after={}&per_page={}",
        start.unwrap_or_else(|| user.last_activity()),
        per_page
    ))?;
    Ok(serde_json::from_str::<Vec<StravaActivity>>(&r)?)
}

pub fn sync(mut e: webhook::Event, user: &User) -> TbResult<Summary> {
    // let mut len = batch;
    let mut start = e.event_time;
    let mut hash = SumHash::default();

    // while len == batch 
    {
        let acts = next_activities(&user, 10, Some(start))?;
        if acts.len() == 0 {
            e.delete(user.conn())?;
        } else {
            for a in acts {
                start = std::cmp::max(start, a.start_date.timestamp());
                trace!("processing sync event at {}", start);
                let ps = a.send_to_tb(&user)?;
                e.setdate(start,  user.conn())?;
                hash.merge(ps);
            }
        }
    }

    Ok(hash.collect())
}