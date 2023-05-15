use chrono::{DateTime, Utc};

use super::*;
use ActivityId;
use NewActivity;
use ActTypeId;

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
    fn into_tb(self, context: &dyn StravaContext) -> Result<NewActivity> {
        let what = self.what()?;
        let gear = match self.gear_id {
            Some(x) => Some(gear::strava_to_tb(x, context)?),
            None => None,
        };
        let (user, _) = context.split();
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
    fn what(&self) -> Result<ActTypeId> {
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
    pub fn send_to_tb(self, context: &dyn StravaContext) -> Result<Summary> {
        let (user, conn) = context.split();
        conn.transaction(||{
            use schema::strava_activities::dsl::*;

            let strava_id = self.id;
            let tb = self.into_tb(context)?;

            let tb_id = strava_activities
                .find(strava_id)
                .select(tendabike_id)
                .for_update()
                .get_result::<ActivityId>(conn)
                .optional()?;

            let res; 
            if let Some(tb_id) = tb_id {
                res = tb_id.update(&tb, user, conn)?
            } else {
                res = Activity::create(&tb, user, conn)?;
                let new_id = res.first();
                diesel::insert_into(strava_activities)
                    .values((
                        id.eq(strava_id),
                        tendabike_id.eq(new_id),
                        user_id.eq(tb.user_id),
                    ))
                    .execute(conn)?;
            }

            user.update_last(tb.start.timestamp(), conn)
                .context("unable to update user")?;

            Ok(res)
        })
    }
}

pub fn strava_url(act: i32, context: &dyn StravaContext) -> Result<String> {
    use schema::strava_activities::dsl::*;

    let g: i64 = strava_activities
        .filter(tendabike_id.eq(act))
        .select(id)
        .first(context.conn())?;

    Ok(format!("https://strava.com/activities/{}", &g))
}

fn get_activity(id: i64, context: &dyn StravaContext) -> Result<StravaActivity> {
    let r = context.request(&format!("/activities/{}",id ))?;
    // let r = user.request("/activities?per_page=2")?;
    let act: StravaActivity = serde_json::from_str(&r)?;
    Ok(act)
}

pub fn upsert_activity(id: i64, context: &dyn StravaContext) -> Result<Summary> {
    let act = get_activity(id, context).context(format!("strava activity id {}", id))?;
    act.send_to_tb(context)
}

pub fn delete_activity(sid: i64, context: &dyn StravaContext) -> Result<Summary> {
    use schema::strava_activities::dsl::*;

    let (user, conn) = context.split();
    conn.transaction(||{
        let tid: Option<ActivityId> = strava_activities.select(tendabike_id).find(sid).for_update().first(conn).optional()?;
        if let Some(tid) = tid {
            diesel::delete(strava_activities.find(sid)).execute(conn)?;
            tid.delete(user, conn)
        } else {
            Ok(Summary::default())
        }
    })
}

