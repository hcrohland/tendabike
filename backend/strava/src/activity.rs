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
    async fn into_activity(
        self,
        user: &mut impl StravaSession,
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
            device_name,
            external_id,
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
            device_name,
            external_id,
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
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let activity = self.into_activity(user, store).await?;

        activity.upsert(user, store).await
    }
}

pub async fn strava_url(
    act: i64,
    user: &impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<String> {
    let g = ActivityId::new(act).read(user, store).await?;
    Ok(format!("https://strava.com/activities/{}", g.id))
}

pub async fn upsert_activity(
    id: i64,
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    let act: StravaActivity = user
        .request_json(&format!("/activities/{id}"), store)
        .await?;
    act.send_to_tb(user, store).await
}

pub(crate) async fn delete_activity(
    act: i64,
    user: &impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    ActivityId::new(act).delete(user, store).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        TestStravaSession, TestStravaStore, activity_json, gear_json, strava_user,
    };

    fn setup() -> (TestStravaStore, TestStravaSession) {
        let mut store = TestStravaStore::new();
        store.insert_user(strava_user(UserId::from(1), 42, true));
        let session = TestStravaSession::new(UserId::from(1), 42.into());
        (store, session)
    }

    fn strava_activity(gear: Option<&str>) -> StravaActivity {
        serde_json::from_str(&activity_json(10, "Ride", gear)).unwrap()
    }

    #[test]
    fn get_type_maps_strava_types() {
        assert_eq!(StravaActivity::get_type("Ride").unwrap(), 1.into());
        assert_eq!(StravaActivity::get_type("VirtualRide").unwrap(), 5.into());
        assert_eq!(StravaActivity::get_type("EBikeRide").unwrap(), 9.into());
        assert_eq!(StravaActivity::get_type("Run").unwrap(), 3.into());
        assert_eq!(StravaActivity::get_type("VirtualRun").unwrap(), 3.into());
        assert_eq!(StravaActivity::get_type("Workout").unwrap(), 0.into());
        assert_eq!(StravaActivity::get_type("Golf").unwrap(), 0.into());
    }

    #[test]
    fn get_type_rejects_unknown() {
        assert!(matches!(
            StravaActivity::get_type("Frobnicate"),
            Err(Error::BadRequest(_))
        ));
    }

    #[tokio::test]
    async fn into_activity_maps_fields() -> TbResult<()> {
        let (mut store, mut session) = setup();
        let act = strava_activity(None)
            .into_activity(&mut session, &mut store)
            .await?;
        assert_eq!(act.id, ActivityId::new(10));
        assert_eq!(act.what, 1.into());
        assert_eq!(act.user_id, UserId::from(1));
        assert!(act.gear.is_none());
        assert_eq!(act.name, "Test Ride");
        assert_eq!(act.start.unix_timestamp(), 1767348000);
        assert_eq!(act.duration, 3600);
        assert_eq!(act.time, Some(3300));
        assert_eq!(act.distance, Some(25000));
        assert_eq!(act.climb, Some(300));
        assert!(act.descend.is_none());
        assert_eq!(act.energy, Some(4000));
        assert_eq!(act.device_name.as_deref(), Some("Test Device"));
        assert!(act.external_id.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn into_activity_creates_missing_gear() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/gear/b1", &gear_json("b1", Some(0)));
        let act = strava_activity(Some("b1"))
            .into_activity(&mut session, &mut store)
            .await?;
        let part = PartStore::partid_get_part(&mut store.mem, act.gear.unwrap()).await?;
        assert_eq!(part.source.as_deref(), Some("b1"));
        assert_eq!(part.what, 1.into());
        assert_eq!(part.name, "Test Bike");
        assert_eq!(part.vendor, "Test Brand");
        assert_eq!(part.model, "Test Model");
        assert_eq!(session.requests, vec!["/gear/b1".to_string()]);
        Ok(())
    }

    #[tokio::test]
    async fn into_activity_reuses_existing_gear() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/gear/b1", &gear_json("b1", Some(0)));
        let first = strava_activity(Some("b1"))
            .into_activity(&mut session, &mut store)
            .await?;
        let second = strava_activity(Some("b1"))
            .into_activity(&mut session, &mut store)
            .await?;
        assert_eq!(first.gear, second.gear);
        let requests = session
            .requests
            .iter()
            .filter(|r| r.as_str() == "/gear/b1")
            .count();
        assert_eq!(requests, 1);
        Ok(())
    }

    #[tokio::test]
    async fn upsert_activity_stores_new() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        let summary = upsert_activity(10, &mut session, &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        assert_eq!(summary.activities[0].id, ActivityId::new(10));
        let acts = ActivityStore::get_all(&mut store.mem, &UserId::from(1)).await?;
        assert_eq!(acts.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn upsert_activity_updates_existing() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        upsert_activity(10, &mut session, &mut store).await?;
        let json = activity_json(10, "Ride", None).replace("25000.0", "30000.0");
        session.queue("/activities/10", &json);
        upsert_activity(10, &mut session, &mut store).await?;
        let acts = ActivityStore::get_all(&mut store.mem, &UserId::from(1)).await?;
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].distance, Some(30000));
        Ok(())
    }

    #[tokio::test]
    async fn upsert_activity_propagates_try_again() {
        let (mut store, mut session) = setup();
        session.queue_error("/activities/10", Error::TryAgain("rate limited"));
        let res = upsert_activity(10, &mut session, &mut store).await;
        assert!(matches!(res, Err(Error::TryAgain(_))));
    }

    #[tokio::test]
    async fn activity_strava_url() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        upsert_activity(10, &mut session, &mut store).await?;
        let url = strava_url(10, &session, &mut store).await?;
        assert_eq!(url, "https://strava.com/activities/10");
        Ok(())
    }

    #[tokio::test]
    async fn activity_strava_url_missing() {
        let (mut store, session) = setup();
        let res = strava_url(99, &session, &mut store).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_activity_removes() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        upsert_activity(10, &mut session, &mut store).await?;
        delete_activity(10, &session, &mut store).await?;
        let acts = ActivityStore::get_all(&mut store.mem, &UserId::from(1)).await?;
        assert!(acts.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn delete_activity_missing() {
        let (mut store, session) = setup();
        let res = delete_activity(42, &session, &mut store).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_activity_forbidden_for_other_user() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        upsert_activity(10, &mut session, &mut store).await?;
        let other = TestStravaSession::new(UserId::from(2), 43.into());
        let res = delete_activity(10, &other, &mut store).await;
        assert!(matches!(res, Err(Error::Forbidden(_))));
        Ok(())
    }
}
