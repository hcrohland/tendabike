use chrono::{DateTime, Utc};

use crate::*;
use auth::User;
use diesel::prelude::*;

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
            user_id: user.id(),
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
            "VirtualRide"   => Ok(1),
            "EBikeRide"     => Ok(1),
            "AlpineSki"     => Ok(6),
            "Snowboard"     => Ok(2),
            "Run"           => Ok(3),
            "VirtualRun"    => Ok(3),
            "Hike"          => Ok(4),
            "Walk"          => Ok(8),
            "Workout"       => Ok(9),
            "BackcountrySki" => Ok(10),
            "StandUpPaddling" => Ok(11),
            "Windsurf"      => Ok(12),
            "Kitesurf"      => Ok(13),
            "Rowing"        => Ok(14),
            "WaterSport"    => Ok(15),
            _ => bail!("unsupported activity {}", t)
/*  
            "Handcycle" => ,
            "Canoeing" => ,
            "Crossfit" => ,
            "Elliptical" => ,
            "Golf" => ,
            "IceSkate" => ,
            "InlineSkate" => ,
            "Kayaking" => ,
            "NordicSki" => ,
            "RockClimbing" => ,
            "RollerSki" => ,
            "Sail" => ,
            "Skateboard" => ,
            "Snowshoe" => ,
            "Soccer" => ,
            "StairStepper" => ,
            "Surfing" => ,
            "Swim" => ,
            "Velomobile" => ,
            "WeightTraining" => ,
            "Wheelchair" => ,
            "Yoga" => */
        }
    }
}

impl StravaActivity {
    pub fn send_to_tb(self, user: &User) -> TbResult<serde_json::Value> {
        use schema::activities::dsl::*;

        let client = reqwest::Client::new();
        let strava_id = self.id;
        let tb = self.into_tb(user)?;

        let tb_id = activities
            .find(strava_id)
            .select(tendabike_id)
            .get_results::<i32>(user.conn())?
            .pop();
        let client = if let Some(tb_id) = tb_id {
            client.put(&format!("{}/{}/{}", TB_URI, "activ", tb_id))
        } else {
            client.post(&format!("{}{}", TB_URI, "/activ"))
        };

        let res: serde_json::Value = client
            .bearer_auth(&user.token)
            .json(&tb)
            .send().context("unable to contact backend")?
            .error_for_status().context("backend responded with error")?
            .json().context("malformed body")?;

        let new_id = res[0]["id"]
            .as_i64()
            .ok_or_else(|| anyhow!("id is no int {:?}", res[0]))? as i32;
        if tb_id.is_none() {
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
