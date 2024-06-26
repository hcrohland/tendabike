// This file contains the implementation of the StravaActivity struct and its methods.
// StravaActivity is a struct that represents an activity from Strava API.
// It has fields that represent the activity's properties such as id, type, name, start date, elapsed time, moving time, distance, total elevation gain, average watts, and gear id.
// The struct also has a method called into_tb that converts the StravaActivity into a NewActivity struct which is used to create a new activity in the Tendabike API.
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
    /// Converts a StravaActivity into a NewActivity struct which is used to create a new activity in the Tendabike API.
    ///
    /// # Arguments
    ///
    /// * `self` - A StravaActivity struct that represents an activity from Strava API.
    /// * `user` - A reference to a StravaUser struct that represents the user who performed the activity.
    /// * `store` - A mutable reference to an AppConn struct that represents a connection to the Tendabike API.
    ///
    /// # Returns
    ///
    /// A Result containing a NewActivity struct if the conversion was successful, or an error if it failed.
    async fn into_tb(
        self,
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<NewActivity> {
        let mut offset = self.utc_offset as i32;
        if offset % 1800 != 0 {
            offset = offset + 900 - (offset + 900) % 1800;
            warn!("rounding utc_offset for {self:?} to {offset}");
        }
        let offset = UtcOffset::from_whole_seconds(offset).context("Utc Offset invalid")?;
        let what = self.what()?;
        let gear = match self.gear_id {
            Some(x) => Some(gear::strava_to_tb(x, user, store).await?),
            None => None,
        };
        Ok(NewActivity {
            what,
            gear,
            user_id: user.tb_id(),
            name: self.name,
            start: self.start_date.to_offset(offset),
            duration: self.elapsed_time,
            time: Some(self.moving_time),
            distance: Some(self.distance.round() as i32),
            climb: Some(self.total_elevation_gain.round() as i32),
            descend: None,
            energy: self.kilojoules.map(|e| e.round() as i32),
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
    fn what(&self) -> TbResult<ActTypeId> {
        let t = self.type_.as_str();

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
            _ => return Err(Error::BadRequest(format!("unsupported activity {}", t))),
        }
        .into())
    }
}

impl StravaActivity {
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
        migrate: bool,
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let strava_id = self.id;
        let tb = self.into_tb(user, store).await?;
        store
            .transaction(|store| {
                async {
                    let tb_id = store.strava_activity_get_tbid(strava_id).await?;
                    let time = tb.start.unix_timestamp();

                    let res;
                    if let Some(tb_id) = tb_id {
                        res = match migrate {
                            true => tb_id.migrate(tb, user, store).await,
                            _ => tb_id.update(tb, user, store).await,
                        }?;
                    } else {
                        res = Activity::create(&tb, user, store).await?;
                        let new_id = res.activities[0].id;
                        store
                            .strava_activity_new(strava_id, tb.user_id, new_id)
                            .await?;
                    }

                    user.strava_id()
                        .update_last(time, store)
                        .await
                        .context("unable to update user")?;

                    Ok(res)
                }
                .scope_boxed()
            })
            .await
    }
}

pub async fn strava_url(act: i32, store: &mut impl StravaStore) -> TbResult<String> {
    let g = store.strava_activitid_get_by_tbid(act).await?;

    Ok(format!("https://strava.com/activities/{}", &g))
}

pub async fn upsert_activity(
    id: i64,
    user: &mut impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    let act: StravaActivity = user
        .request_json(&format!("/activities/{}", id), store)
        .await?;
    act.send_to_tb(false, user, store).await
}

pub(crate) async fn delete_activity(
    act_id: i64,
    user: &impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    let tid = store.strava_activity_get_activityid(act_id).await?;
    let mut res = Summary::default();
    if let Some(tid) = tid {
        // first delete the tendabike activity
        res = tid.delete(user, store).await?;
        // now delete the reference to the strava activity
        // if this fails we end up with an orphaned entry in the strava_activities table, which should not be a problem in practice
        store.strava_activity_delete(act_id).await?;
    }
    Ok(res)
}
