// This file contains the implementation of the StravaActivity struct and its methods.
// StravaActivity is a struct that represents an activity from Strava API.
// It has fields that represent the activity's properties such as id, type, name, start date, elapsed time, moving time, distance, total elevation gain, average watts, and gear id.
// The struct also has a method called into_tb that converts the StravaActivity into a Activity struct which is used to create a new activity in the Tendabike API.
// The struct also has a method called what that maps Strava workout type strings to Tendabike types.
// The file imports the OffsetDateTime struct from the time crate.
// The file also has two comments that indicate the beginning and end of a code block.

use time::{OffsetDateTime, UtcOffset};

use super::*;
use crate::{EntityFacade, ProviderActivityId, StravaActivityId};

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
    async fn into_activity<D: EntityFacade<S>, S: StravaDataStore>(
        self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
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
            device_name,
            external_id,
        } = self;
        let offset =
            UtcOffset::from_whole_seconds(utc_offset as i32).context("Utc Offset invalid")?;
        let what = Self::get_type(&type_)?;
        let gear = match gear_id {
            // cannot use map due to async closure
            Some(x) => Some(gear::into_partid(x, user_id, facade, store).await?),
            None => None,
        };
        Ok(Activity {
            id: ProviderActivityId::Strava(StravaActivityId::from(id)).into(),
            what,
            gear,
            user_id,
            name,
            start: start_date.to_offset(offset),
            duration: elapsed_time,
            time: Some(moving_time),
            distance: Some(distance.round() as i32),
            climb: Some(total_elevation_gain.round() as i32),
            descend: None,
            energy: kilojoules.map(|e| e.round() as i32),
            device_name,
            external_id,
        })
    }

    /// Maps Strava workout type strings to Tendabike types: the common
    /// wire-name arms from the provider kit's table
    /// ([`crate::activity_type::common_type`]). Strava carries no
    /// cycling-subtype arms of its own, so a miss is an unsupported type.
    fn get_type(t: &str) -> TbResult<ActTypeId> {
        crate::activity_type::common_type(t)
            .ok_or_else(|| Error::BadRequest(format!("unsupported activity {t}")))
    }

    /// Sends the activity to Tendabike API.
    pub(crate) async fn send_to_tb<D: EntityFacade<S>, S: StravaDataStore>(
        self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        let activity = self.into_activity(user_id, facade, store).await?;
        let session = Box::new(OnBehalf(user_id));
        facade.upsert_activity(activity, session, store).await
    }
}

/// The Strava URL for a Strava activity: the driver extracts the raw
/// (unprefixed) Strava id from the domain's opaque [`ActivityId`], the read
/// is the ownership check, and the URL is built from the raw id.
pub async fn strava_url<D: EntityFacade<S>, S: StravaDataStore>(
    id: ActivityId,
    user: Box<dyn Session>,
    facade: &D,
    store: &mut S,
) -> TbResult<String> {
    let ProviderActivityId::Strava(raw) = ProviderActivityId::try_from(id)? else {
        return Err(Error::BadRequest("not a strava activity".to_string()));
    };
    let full: ActivityId = ProviderActivityId::Strava(raw).into();
    facade.read_activity(&full, user, store).await?;
    Ok(format!("https://strava.com/activities/{raw}"))
}

pub(crate) async fn upsert_activity<D: EntityFacade<S>, S: StravaDataStore>(
    id: i64,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    let act: StravaActivity = request_json(user_id, &format!("/activities/{id}"), store).await?;
    act.send_to_tb(user_id, facade, store).await
}

pub(crate) async fn delete_activity<D: EntityFacade<S>, S: StravaDataStore>(
    act: i64,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    let session = Box::new(OnBehalf(user_id));
    facade
        .delete_activity(
            ProviderActivityId::Strava(StravaActivityId::from(act)).into(),
            session,
            store,
        )
        .await
}

#[cfg(test)]
mod activity_tests;
