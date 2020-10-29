use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::*;
use auth::User;
use diesel::prelude::*;
use reqwest::blocking::Client;

#[derive(Debug, Default)]
struct SumHash {
    activities: HashMap<Option<i64>, jValue>,
    parts: HashMap<Option<i64>, jValue>,
    atts: HashMap<String, jValue>,
}

impl SumHash {
    fn merge(&mut self, ps: JSummary)  {
        for act in ps.activities {
            self.activities.insert(act["id"].as_i64(), act);
        }
        for part in ps.parts {
            self.parts.insert(part["id"].as_i64(), part);
        }
        for att in ps.attachments {
            self.atts.insert(format!("{}{}",att["part_id"],att["attached"]), att);
        }
    }

    fn collect(self) -> JSummary {
        JSummary {
            activities: self.activities.into_iter().map(|(_,v)| v).collect(),
            parts: self.parts.into_iter().map(|(_,v)| v).collect(),
            attachments: self.atts.into_iter().map(|(_,v)| v).collect(),
        }
    }
}

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

#[derive(Serialize, Deserialize, Debug)]
pub struct TbActivity {
    pub user_id: i32,
    /// The activity type
    pub what: i32,
    /// This name of the activity.
    pub name: String,
    /// Start time
    pub start: DateTime<Utc>,
    /// End time
    pub duration: i32,
    /// activity time
    pub time: i32,
    /// Covered distance
    pub distance: i32,
    /// Total climbing
    pub climb: i32,
    /// Total descending
    pub descend: Option<i32>,
    /// average power output
    pub power: Option<i32>,
    /// Which gear did she use?
    pub gear: Option<i32>,
}

impl StravaActivity {
    pub fn into_tb(self, user: &User) -> TbResult<TbActivity> {
        let what = self.what()?;
        let gear = match self.gear_id {
            Some(x) => Some(gear::strava_to_tb(x, user)?),
            None => None,
        };
        Ok(TbActivity {
            what,
            gear,
            user_id: user.tb_id(),
            name: self.name,
            start: self.start_date,
            duration: self.elapsed_time,
            time: self.moving_time,
            distance: self.distance.round() as i32,
            climb: self.total_elevation_gain.round() as i32,
            descend: None,
            power: self.average_watts.map(|p| p.round() as i32),
        })
    }

    /// map strava workout type strings to tendabike types
    fn what(&self) -> TbResult<i32> {
        let t = self.type_.as_str();

        match t {
            "Ride"          => Ok(1),
            "VirtualRide"   => Ok(5),
            "EBikeRide"     => Ok(9),
            "Snowboard"     => Ok(2),
            "Run"           => Ok(3),
            "VirtualRun"    => Ok(3),
            "Hike"          => Ok(4),
            "AlpineSki"     => Ok(6),
            "Walk"          => Ok(8),
            "BackcountrySki" => Ok(10),
            "Workout"       => Ok(0),
            "StandUpPaddling" => Ok(0),
            "Windsurf"      => Ok(0),
            "Kitesurf"      => Ok(0),
            "Rowing"        => Ok(0),
            "WaterSport"    => Ok(0),
            "RockClimbing"  => Ok(0),
            "Handcycle" => Ok(0),
            "Canoeing" => Ok(0),
            "Crossfit" => Ok(0),
            "Elliptical" => Ok(0),
            "Golf" => Ok(0),
            "IceSkate" => Ok(0),
            "InlineSkate" => Ok(0),
            "Kayaking" => Ok(0),
            "NordicSki" => Ok(0),
            "RollerSki" => Ok(0),
            "Sail" => Ok(0),
            "Skateboard" => Ok(0),
            "Snowshoe" => Ok(0),
            "Soccer" => Ok(0),
            "StairStepper" => Ok(0),
            "Surfing" => Ok(0),
            "Swim" => Ok(0),
            "Velomobile" => Ok(0),
            "WeightTraining" => Ok(0),
            "Wheelchair" => Ok(0),
            "Yoga" => Ok(0),
            _ => bail!("unsupported activity {}", t)
        }
    }
}

impl StravaActivity {
    pub fn send_to_tb(self, user: &User) -> TbResult<JSummary> {
        use schema::activities::dsl::*;

        let client = Client::new();
        let strava_id = self.id;
        let tb = self.into_tb(user)?;

        let tb_id = activities
            .find(strava_id)
            .select(tendabike_id)
            .get_result::<i32>(user.conn())
            .optional()?;
        let client = if let Some(tb_id) = tb_id {
            client.put(&format!("{}/{}/{}", user.url, "activ", tb_id))
        } else {
            client.post(&format!("{}/{}", user.url, "activ"))
        };

        let res: JSummary = client
            .bearer_auth(&user.token)
            .json(&tb)
            .send().context("unable to contact backend")?
            .error_for_status().context("backend responded with error")?
            .json().context("malformed body")?;

        if tb_id.is_none() {
            let act = &res.activities[0];
            let new_id = act["id"]
                .as_i64()
                .ok_or_else(|| anyhow!("id is no int {:?}", act))? as i32;
            diesel::insert_into(activities)
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
    }
}

pub(crate) fn strava_url(act: i32, user: &User) -> TbResult<String> {
    use schema::activities::dsl::*;

    let g: i64 = activities
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

fn upsert_activity(id: i64, user: &User) -> TbResult<JSummary> {
    let act = get_activity(id, user)?;
    let ps = act.send_to_tb(user)?;
    Ok(ps)
}

fn delete_activity(sid: i64, user: &User) -> TbResult<JSummary> {
    use schema::activities::dsl::*;

    user.conn().transaction(||{
        let tid: Option<i32> = activities.select(tendabike_id).find(sid).for_update().first(user.conn()).optional()?;
        if let Some(tid) = tid {
            diesel::delete(activities.find(sid)).execute(user.conn())?;
            return Ok(
                Client::new()   
                    .delete(&format!("{}/{}/{}", user.url, "activ", tid))
                    .bearer_auth(&user.token)
                    .send().context("unable to contact backend")?
                    .error_for_status().context("backend responded with error")?
                    .json().context("malformed body")?
            );
        } else {
            return Ok(JSummary::default());
        };
    })
}

pub fn process_hook(e: webhook::Event, user: &User) -> TbResult<JSummary>{
    debug!("Processing event {:?}", e);
    let res = match e.aspect_type.as_str() {
        "create" | "update" => upsert_activity(e.object_id, user)?,
        "delete" => delete_activity(e.object_id, user)?,
        "sync" =>  return sync(e, user),
        _ => {
            warn!("Skipping unknown aspect_type {:?}", e);
            JSummary::default()
        }
    };
    e.delete(user)?;
    Ok(res)
}

pub(crate) fn next_activities(user: &User, per_page: usize, start: Option<i64>) -> TbResult<Vec<StravaActivity>> {
    let r = user.request(&format!(
        "/activities?after={}&per_page={}",
        start.unwrap_or_else(|| user.last_activity()),
        per_page
    ))?;
    Ok(serde_json::from_str::<Vec<StravaActivity>>(&r)?)
}

pub(crate) fn sync(e: webhook::Event, user: &User) -> TbResult<JSummary> {
    // let mut len = batch;
    let mut start = user.last_activity();
    let mut hash = SumHash::default();

    // while len == batch 
    {
        let acts = next_activities(&user, 10, Some(start))?;
        if acts.len() == 0 {
            e.delete(user)?;
        } else {
            for a in acts {
                start = std::cmp::max(start, a.start_date.timestamp());
                let ps = a.send_to_tb(&user)?;
                hash.merge(ps);
            }
        }
    }

    Ok(hash.collect())
}