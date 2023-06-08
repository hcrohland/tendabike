use async_trait::async_trait;
// use domain::{ActivityId, AnyResult, NewPart, PartId, Person, UserId};
use domain::{AnyResult, PartId, ActivityId, UserId};

use crate::{event::Event, StravaId, StravaUser};
// use crate::{event::Event, StravaId, StravaUser};

#[async_trait]
pub trait StravaStore: domain::traits::Store + Send {
    async fn stravaid_get_user_id(&mut self, who: i32) -> AnyResult<i32>;

    async fn strava_gear_get_tbid(&mut self, strava_id: &str) -> AnyResult<Option<PartId>>;

    async fn strava_gearid_get_name(&mut self, gear: i32) -> AnyResult<String>;

    async fn strava_activity_get_tbid(
        &mut self,
        strava_id: i64,
    ) -> AnyResult<Option<ActivityId>>;

    async fn strava_activity_new(
        &mut self,
        strava_id: i64,
        uid: UserId,
        new_id: ActivityId,
    ) -> AnyResult<()>;

    async fn strava_activitid_get_by_tbid(&mut self, act: i32) -> Result<i64, anyhow::Error>;

    async fn strava_activity_delete(&mut self, act_id: i64) -> AnyResult<usize>;

    async fn strava_activity_get_activityid(
        &mut self,
        act_id: i64,
    ) -> AnyResult<Option<ActivityId>>;

    async fn strava_gear_new(
        &mut self,
        strava_id: String,
        tbid: PartId,
        user: UserId,
    )-> AnyResult<()>;

    async fn strava_event_delete(
        &mut self,
        event_id: Option<i32>,
    ) -> AnyResult<()>;

    async fn strava_event_set_time(
        &mut self,
        e_id: Option<i32>,
        e_time: i64,
    ) -> AnyResult<()>;

    async fn stravaevent_store(&mut self, e: Event) -> AnyResult<()>;

    async fn strava_event_get_next_for_user(
        &mut self,
        user: &crate::StravaUser,
    ) -> AnyResult<Option<Event>>;

    async fn strava_event_get_later(
        &mut self,
        obj_id: i64,
        oid: StravaId,
    ) -> AnyResult<Vec<Event>>;

    async fn strava_events_delete_batch(
        &mut self,
        values: Vec<Option<i32>>,
    ) -> AnyResult<()>;

    async fn stravausers_get_all(&mut self) -> AnyResult<Vec<StravaUser>>;

    async fn stravauser_get_by_tbid(
        &mut self,
        id: UserId,
    ) -> AnyResult<StravaUser>;

    async fn stravauser_get_by_stravaid(
        &mut self,
        id: StravaId,
    ) -> AnyResult<Vec<StravaUser>>;

    async fn stravauser_new(
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

    async fn strava_events_get_count_for_user(&mut self, user: &StravaUser) -> AnyResult<i64>;

    async fn stravauser_disable(&mut self, user: &StravaId) -> AnyResult<()>;

    async fn stravaid_lock(&mut self, user_id: &StravaId) -> AnyResult<bool>;
    async fn stravaid_unlock(&mut self, id: StravaId) -> AnyResult<usize>;

}
