use crate::*;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_async::RunQueryDsl;
use tb_domain::ActivityId;
use tb_domain::PartId;
use tb_domain::TbResult;
use tb_domain::UserId;
use tb_strava::StravaPerson;
use tb_strava::event::Event;
use tb_strava::schema;
use tb_strava::StravaId;
use tb_strava::StravaUser;

#[async_session::async_trait]
impl tb_strava::StravaStore for AsyncDieselConn {
    async fn stravaid_get_user_id(&mut self, who: i32) -> TbResult<i32> {
        use schema::strava_users::dsl::*;
        strava_users
            .filter(tendabike_id.eq(who))
            .select(id)
            .first(self)
            .await
            .map_err(map_to_tb)
    }

    async fn stravaevent_store(&mut self, e: Event) -> TbResult<()> {
        diesel::insert_into(schema::strava_events::table)
            .values(&e)
            .get_result::<Event>(&mut self)
            .await?;
        Ok(())
    }

    async fn strava_gear_get_tbid(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        use schema::strava_gears::dsl::*;

        strava_gears
            .find(strava_id)
            .select(tendabike_id)
            .for_update()
            .first(self)
            .await
            .optional()
            .map_err(map_to_tb)
    }

    async fn strava_gearid_get_name(&mut self, gear: i32) -> TbResult<String> {
        use schema::strava_gears::dsl::*;
        strava_gears
            .filter(tendabike_id.eq(gear))
            .select(id)
            .first(self)
            .await
            .map_err(map_to_tb)
    }

    async fn strava_activity_get_tbid(&mut self, strava_id: i64) -> TbResult<Option<ActivityId>> {
        use schema::strava_activities::dsl::*;

        strava_activities
            .find(strava_id)
            .select(tendabike_id)
            .for_update()
            .get_result::<ActivityId>(self)
            .await
            .optional()
            .map_err(map_to_tb)
    }

    async fn strava_activity_new(
        &mut self,
        strava_id: i64,
        uid: UserId,
        new_id: ActivityId,
    ) -> TbResult<()> {
        use schema::strava_activities::dsl::*;

        diesel::insert_into(strava_activities)
            .values((id.eq(strava_id), tendabike_id.eq(new_id), user_id.eq(uid)))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn strava_activitid_get_by_tbid(&mut self, act: i32) -> TbResult<i64> {
        use schema::strava_activities::dsl::*;
        strava_activities
            .filter(tendabike_id.eq(act))
            .select(id)
            .first(self)
            .await
            .map_err(map_to_tb)
    }

    async fn strava_activity_delete(&mut self, act_id: i64) -> TbResult<usize> {
        use schema::strava_activities::dsl::*;
        diesel::delete(strava_activities.find(act_id))
            .execute(self)
            .await
            .map_err(map_to_tb)
    }

    async fn strava_activity_get_activityid(
        &mut self,
        act_id: i64,
    ) -> TbResult<Option<ActivityId>> {
        use schema::strava_activities::dsl::*;
        strava_activities
            .find(act_id)
            .select(tendabike_id)
            .first(self)
            .await
            .optional()
            .map_err(map_to_tb)
    }

    async fn strava_gear_new(
        &mut self,
        strava_id: String,
        tbid: PartId,
        user: UserId,
    ) -> TbResult<()> {
        use schema::strava_gears::dsl::*;

        diesel::insert_into(strava_gears)
            .values((id.eq(strava_id), tendabike_id.eq(tbid), user_id.eq(user)))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        use schema::strava_events::dsl::*;
        diesel::delete(strava_events)
            .filter(id.eq(event_id))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn strava_event_set_time(
        &mut self,
        e_id: Option<i32>,
        e_time: i64,
    ) -> TbResult<()> {
        use schema::strava_events::dsl::*;
        diesel::update(strava_events)
            .filter(id.eq(e_id))
            .set(event_time.eq(e_time))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn strava_event_get_next_for_user(
        &mut self,
        user: &impl StravaPerson,
    ) -> TbResult<Option<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(owner_id.eq_any(vec![0, user.strava_id().into()]))
            .first::<Event>(self)
            .await
            .optional()
            .map_err(map_to_tb)
    }

    async fn strava_event_get_later(
        &mut self,
        obj_id: i64,
        oid: StravaId,
    ) -> TbResult<Vec<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(object_id.eq(obj_id))
            .filter(owner_id.eq(oid))
            .order(event_time.asc())
            .get_results::<Event>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        use schema::strava_events::dsl::*;

        diesel::delete(strava_events)
            .filter(id.eq_any(values))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        schema::strava_users::table
            .get_results::<StravaUser>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn stravauser_get_by_tbid(
        &mut self,
        id: UserId,
    ) -> TbResult<StravaUser> {
        schema::strava_users::table
            .filter(schema::strava_users::tendabike_id.eq(id))
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn stravauser_get_by_stravaid(
        &mut self,
        id: &StravaId,
    ) -> TbResult<Option<StravaUser>> {
        schema::strava_users::table
            .find(id)
            .first::<StravaUser>(self)
            .await
            .optional()
            .map_err(map_to_tb)
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        diesel::insert_into(schema::strava_users::table)
            .values(&user)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn stravauser_update_last_activity(
        &mut self,
        user: &StravaId,
        time: i64,
    ) -> TbResult<()> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(user))
            .set(last_activity.eq(time))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> TbResult<StravaUser> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(stravaid))
            .set((
                refresh_token.eq(refresh),
            ))
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    /// return the open events and the disabled status for a user.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database connection fails.
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64> {
        use schema::strava_events::dsl::*;

        strava_events
            .count()
            .filter(owner_id.eq(user))
            .first(self)
            .await
            .map_err(map_to_tb)
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        use schema::strava_events::dsl::*;

        diesel::delete(strava_events.filter(owner_id.eq(user)))
            .execute(self)
            .await
            .map_err(map_to_tb)
    }

    async fn stravaid_lock(&mut self, user_id: &StravaId) -> TbResult<bool> {
        use diesel::sql_types::Bool;
        #[derive(QueryableByName, Debug)]
        struct Lock {
            #[diesel(sql_type = Bool)]
            #[diesel(column_name = pg_try_advisory_lock)]
            lock: bool,
        }
        let lock: bool = sql_query(format!("SELECT pg_try_advisory_lock({});", user_id))
            .get_result::<Lock>(self)
            .await?
            .lock;
        Ok(lock)
    }

    async fn stravaid_unlock(&mut self, id: &StravaId) -> TbResult<usize> {
        sql_query(format!("SELECT pg_advisory_unlock({});", id))
            .execute(self)
            .await
            .map_err(map_to_tb)
    }
}
