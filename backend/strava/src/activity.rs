// This file contains the implementation of the StravaActivity struct and its methods.
// StravaActivity is a struct that represents an activity from Strava API.
// It has fields that represent the activity's properties such as id, type, name, start date, elapsed time, moving time, distance, total elevation gain, average watts, and gear id.
// The struct also has a method called into_tb that converts the StravaActivity into a Activity struct which is used to create a new activity in the Tendabike API.
// The struct also has a method called what that maps Strava workout type strings to Tendabike types.
// The file imports the OffsetDateTime struct from the time crate.
// The file also has two comments that indicate the beginning and end of a code block.

use time::{OffsetDateTime, UtcOffset};

use crate::*;

#[derive(Serialize, Deserialize, Debug)]
/// A struct that represents an activity from Strava API.
/// It has fields that represent the activity's properties such as id, type, name, start date, elapsed time, moving time, distance, total elevation gain, average watts, and gear id.
pub(crate) struct StravaActivity {
    pub id: i64,
    /// The activity type
    #[serde(rename = "type")]
    pub type_: String,
    /// This name of the activity.
    pub name: String,
    /// Start time
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    pub utc_offset: f32,
    /// End time
    pub elapsed_time: i32,
    /// activity time
    pub moving_time: i32,
    /// Covered distance
    pub distance: f64,
    /// Total climbing
    pub total_elevation_gain: f64,
    /// Energy excerted
    pub kilojoules: Option<f64>,
    /// Which gear did she use?
    pub gear_id: Option<String>,
}

impl StravaActivity {
    /// Converts a StravaActivity into a Activity struct which is used to create a new activity in the Tendabike API.
    ///
    /// # Arguments
    ///
    /// * `self` - A StravaActivity struct that represents an activity from Strava API.
    /// * `user` - A reference to a StravaUser struct that represents the user who performed the activity.
    /// * `store` - A mutable reference to an AppConn struct that represents a connection to the Tendabike API.
    ///
    /// # Returns
    ///
    /// A Result containing a Activity struct if the conversion was successful, or an error if it failed.
    async fn into_activity(
        self,
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Activity> {
        let StravaActivity {
            id,
            type_,
            name,
            start_date,
            utc_offset,
            elapsed_time,
            moving_time,
            distance,
            total_elevation_gain,
            kilojoules,
            gear_id,
        } = self;
        let offset =
            UtcOffset::from_whole_seconds(utc_offset as i32).context("Utc Offset invalid")?;
        let what = Self::get_type(&type_)?;
        let gear = match gear_id {
            // cannot use map due to async closure
            Some(x) => Some(gear::into_partid(x, user, store).await?),
            None => None,
        };
        Ok(Activity {
            id: id.into(),
            what,
            gear,
            user_id: user.tb_id(),
            name,
            start: start_date.to_offset(offset),
            duration: elapsed_time,
            time: Some(moving_time),
            distance: Some(distance.round() as i32),
            climb: Some(total_elevation_gain.round() as i32),
            descend: None,
            energy: kilojoules.map(|e| e.round() as i32),
        })
    }

    /// Maps Strava workout type strings to Tendabike types.
    ///
    /// # Arguments
    ///
    /// * `self` - A reference to a StravaActivity struct that represents an activity from Strava API.
    ///
    /// # Returns
    ///
    /// A Result containing an ActTypeId if the mapping was successful, or an error if it failed.
    fn get_type(t: &str) -> TbResult<ActTypeId> {
        Ok(match t {
            "Ride" => 1,
            "VirtualRide" => 5,
            "EBikeRide" => 9,
            "Snowboard" => 2,
            "Run" => 3,
            "VirtualRun" => 3,
            "Hike" => 4,
            "AlpineSki" => 6,
            "Walk" => 8,
            "BackcountrySki" => 10,
            "Workout" => 0,
            "StandUpPaddling" => 0,
            "Windsurf" => 0,
            "Kitesurf" => 0,
            "Rowing" => 0,
            "WaterSport" => 0,
            "RockClimbing" => 0,
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
            _ => return Err(Error::BadRequest(format!("unsupported activity {t}"))),
        }
        .into())
    }

    /// Sends the activity to Tendabike API.
    ///
    /// # Arguments
    ///
    /// * `self` - A reference to a StravaActivity struct that represents an activity from Strava API.
    /// * `user` - A reference to a StravaUser struct that represents the user from Strava API.
    /// * `store` - A mutable reference to an AppConn struct that represents the connection to the Tendabike API.
    ///
    /// # Returns
    ///
    /// A Result containing a Summary if the sending was successful, or an error if it failed.
    pub(crate) async fn send_to_tb(
        self,
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let activity = self.into_activity(user, store).await?;

        activity.upsert(user, store).await
    }
}

pub async fn strava_url(
    act: i64,
    user: &impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<String> {
    let g = ActivityId::new(act).read(user, store).await?;
    Ok(format!("https://strava.com/activities/{}", g.id))
}

pub async fn upsert_activity(
    id: i64,
    user: &mut impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    let act: StravaActivity = user.request_json(&format!("/activities/{id}")).await?;
    act.send_to_tb(user, store).await
}

pub(crate) async fn delete_activity(
    act: i64,
    user: &impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    ActivityId::new(act).delete(user, store).await
}
