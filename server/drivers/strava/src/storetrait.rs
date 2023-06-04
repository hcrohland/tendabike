use async_trait::async_trait;
// use domain::{ActivityId, AnyResult, NewPart, PartId, Person, UserId};
use domain::{AnyResult, PartId, ActivityId, UserId, NewPart, Person};

use crate::{event::Event, StravaId, StravaUser};
// use crate::{event::Event, StravaId, StravaUser};

#[async_trait]
pub trait Store {
    async fn get_user_id_from_strava_id(&mut self, who: i32) -> AnyResult<i32>;

    async fn get_tbid_for_strava_gear(&mut self, strava_id: &str) -> AnyResult<Option<PartId>>;

    async fn get_strava_name_for_gear_id(&mut self, gear: i32) -> AnyResult<String>;

    async fn get_tbid_for_strava_activity(
        &mut self,
        strava_id: i64,
    ) -> AnyResult<Option<ActivityId>>;

    async fn insert_new_activity(
        &mut self,
        strava_id: i64,
        uid: UserId,
        new_id: ActivityId,
    ) -> AnyResult<()>;

    async fn get_stravaid_for_tb_activity(&mut self, act: i32) -> Result<i64, anyhow::Error>;

    async fn delete_strava_activity(&mut self, act_id: i64) -> AnyResult<usize>;

    async fn get_activityid_from_strava_activity(
        &mut self,
        act_id: i64,
    ) -> AnyResult<Option<ActivityId>>;

    async fn create_new_gear(
        &mut self,
        strava_id: String,
        part: NewPart,
        user: &dyn Person,
    ) -> AnyResult<PartId>;

    async fn delete_strava_event(
        &mut self,
        event_id: Option<i32>,
    ) -> AnyResult<()>;

    async fn set_event_time(
        &mut self,
        e_id: Option<i32>,
        e_time: i64,
    ) -> AnyResult<()>;

    async fn store_stravaevent(&mut self, e: Event) -> AnyResult<()>;

    async fn get_next_event_for_stravauser(
        &mut self,
        user: &crate::StravaUser,
    ) -> AnyResult<Option<Event>>;

    async fn get_all_later_events_for_object(
        &mut self,
        obj_id: i64,
        oid: StravaId,
    ) -> AnyResult<Vec<Event>>;

    async fn delete_events_by_vec_id(
        &mut self,
        values: Vec<Option<i32>>,
    ) -> AnyResult<()>;

    async fn get_all_stravausers(&mut self) -> AnyResult<Vec<StravaUser>>;

    async fn read_stravauser_for_userid(
        &mut self,
        id: UserId,
    ) -> AnyResult<StravaUser>;

    async fn read_stravauser_for_stravaid(
        &mut self,
        id: StravaId,
    ) -> AnyResult<Vec<StravaUser>>;

    async fn insert_stravauser(
        &mut self,
        user: StravaUser,
    ) -> AnyResult<StravaUser>;

    async fn stravauser_update_last_activity(
        &mut self,
        user: &StravaUser,
        time: i64,
    ) -> AnyResult<()>;

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        access: &str,
        exp: i64,
        refresh: Option<&str>,
    ) -> AnyResult<StravaUser>;

    async fn get_count_of_events_for_user(&mut self, user: &StravaUser) -> AnyResult<i64>;

    async fn disable_stravauser(&mut self, user: &StravaId) -> AnyResult<()>;


}
