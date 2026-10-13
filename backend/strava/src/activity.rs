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
    /// the original device name
    /// Only provided by the specific activity request
    pub device_name: Option<String>,
    /// An identifier for the original source
    /// Garmin activities seem to start with "garmin..."
    pub external_id: Option<String>,
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
    async fn into_activity<D: Domain, S: Session + StravaSession<D>>(
        self,
        user: &mut S,
        store: &mut D::Store,
    ) -> Result<D::Activity, D::Error>
    where
        D::Store: StravaStore<D>,
    {
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
            device_name,
            external_id,
        } = self;
        let offset = UtcOffset::from_whole_seconds(utc_offset as i32)
            .context("Utc Offset invalid")
            .map_err(D::any_failure)?;
        let what = Self::get_type::<D>(&type_)?;
        let gear = match gear_id {
            // cannot use map due to async closure
            Some(x) => Some(gear::into_partid::<D, _>(x, user, store).await?),
            None => None,
        };
        Ok(D::make_activity(
            D::make_activity_id(id),
            user.tb_id(),
            what,
            name,
            start_date.to_offset(offset),
            elapsed_time,
            Some(moving_time),
            Some(distance.round() as i32),
            Some(total_elevation_gain.round() as i32),
            None,
            kilojoules.map(|e| e.round() as i32),
            gear,
            device_name,
            external_id,
        ))
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
    fn get_type<D: Domain>(t: &str) -> Result<D::ActTypeId, D::Error> {
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
            _ => return Err(D::bad_request(format!("unsupported activity {t}"))),
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
    /// A Result containing a `Summary` map if the sending was successful, or an error if it failed.
    pub(crate) async fn send_to_tb<D: Domain, S: Session + StravaSession<D>>(
        self,
        user: &mut S,
        store: &mut D::Store,
    ) -> Result<D::Summary, D::Error>
    where
        D::Store: StravaStore<D>,
    {
        let activity = self.into_activity::<D, _>(user, store).await?;
        D::upsert_activity(activity, user, store).await
    }
}

pub async fn strava_url<
    St: Store + StravaStore<ConcreteDomain<St>>,
    S: Session + StravaSession<ConcreteDomain<St>>,
>(
    act: i64,
    user: &S,
    store: &mut St,
) -> TbResult<String> {
    let g = ActivityId::new(act).read(user, store).await?;
    Ok(format!("https://strava.com/activities/{}", g.id))
}

pub async fn upsert_activity<D: Domain, S: Session + StravaSession<D>>(
    id: i64,
    user: &mut S,
    store: &mut D::Store,
) -> Result<D::Summary, D::Error>
where
    D::Store: StravaStore<D>,
{
    let act: StravaActivity = user
        .request_json(&format!("/activities/{id}"), store)
        .await?;
    act.send_to_tb::<D, _>(user, store).await
}

pub(crate) async fn delete_activity<D: Domain, S: Session + StravaSession<D>>(
    act: i64,
    user: &S,
    store: &mut D::Store,
) -> Result<D::Summary, D::Error>
where
    D::Store: StravaStore<D>,
{
    D::delete_activity(D::make_activity_id(act), user, store).await
}

#[cfg(test)]
mod activity_tests;
