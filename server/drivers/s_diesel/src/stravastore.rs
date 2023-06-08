use crate::AppConn;
use anyhow::Context;
use anyhow::Result as AnyResult;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_async::RunQueryDsl;
use domain::ActivityId;
use domain::PartId;
use domain::UserId;
use tb_strava::event::Event;
use tb_strava::schema;
use tb_strava::StravaId;
use tb_strava::StravaUser;

#[async_session::async_trait]
impl tb_strava::StravaStore for AppConn {
    async fn stravaid_get_user_id(&mut self, who: i32) -> AnyResult<i32> {
        use schema::strava_users::dsl::*;
        let user_id: i32 = strava_users
            .filter(tendabike_id.eq(who))
            .select(id)
            .first(self)
            .await?;
        Ok(user_id)
    }

    async fn stravaevent_store(&mut self, e: Event) -> AnyResult<()> {
        diesel::insert_into(schema::strava_events::table)
            .values(&e)
            .get_result::<Event>(&mut self)
            .await?;
        Ok(())
    }

    async fn strava_gear_get_tbid(&mut self, strava_id: &str) -> AnyResult<Option<PartId>> {
        use schema::strava_gears::dsl::*;

        strava_gears
            .find(strava_id)
            .select(tendabike_id)
            .for_update()
            .first(self)
            .await
            .optional()
            .context("Error reading database")
    }

    async fn strava_gearid_get_name(&mut self, gear: i32) -> Result<String, anyhow::Error> {
        use schema::strava_gears::dsl::*;
        let g: String = strava_gears
            .filter(tendabike_id.eq(gear))
            .select(id)
            .first(self)
            .await?;
        Ok(g)
    }

    async fn strava_activity_get_tbid(&mut self, strava_id: i64) -> AnyResult<Option<ActivityId>> {
        use schema::strava_activities::dsl::*;

        strava_activities
            .find(strava_id)
            .select(tendabike_id)
            .for_update()
            .get_result::<ActivityId>(self)
            .await
            .optional()
            .context("failed to get tbid for stravaid")
    }

    async fn strava_activity_new(
        &mut self,
        strava_id: i64,
        uid: UserId,
        new_id: ActivityId,
    ) -> AnyResult<()> {
        use schema::strava_activities::dsl::*;

        diesel::insert_into(strava_activities)
            .values((id.eq(strava_id), tendabike_id.eq(new_id), user_id.eq(uid)))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn strava_activitid_get_by_tbid(&mut self, act: i32) -> Result<i64, anyhow::Error> {
        use schema::strava_activities::dsl::*;
        strava_activities
            .filter(tendabike_id.eq(act))
            .select(id)
            .first(self)
            .await
            .context("failed to get stravaid for activity")
    }

    async fn strava_activity_delete(&mut self, act_id: i64) -> Result<usize, anyhow::Error> {
        use schema::strava_activities::dsl::*;
        diesel::delete(strava_activities.find(act_id))
            .execute(self)
            .await
            .context("failed to delete strava activity")
    }

    async fn strava_activity_get_activityid(
        &mut self,
        act_id: i64,
    ) -> Result<Option<ActivityId>, anyhow::Error> {
        use schema::strava_activities::dsl::*;
        strava_activities
            .find(act_id)
            .select(tendabike_id)
            .first(self)
            .await
            .optional()
            .context("failed to get activity id")
    }

    async fn strava_gear_new(
        &mut self,
        strava_id: String,
        tbid: PartId,
        user: UserId,
    ) -> Result<(), anyhow::Error> {
        use schema::strava_gears::dsl::*;

        diesel::insert_into(strava_gears)
            .values((id.eq(strava_id), tendabike_id.eq(tbid), user_id.eq(user)))
            .execute(self)
            .await
            .context("couldn't store gear")?;
        Ok(())
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> Result<(), anyhow::Error> {
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
    ) -> Result<(), anyhow::Error> {
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
        user: &tb_strava::StravaUser,
    ) -> AnyResult<Option<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(owner_id.eq_any(vec![0, user.id.into()]))
            .first::<Event>(self)
            .await
            .optional()
            .context("failed to get next event")
    }

    async fn strava_event_get_later(
        &mut self,
        obj_id: i64,
        oid: StravaId,
    ) -> AnyResult<Vec<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(object_id.eq(obj_id))
            .filter(owner_id.eq(oid))
            .order(event_time.asc())
            .get_results::<Event>(self)
            .await
            .context("failed to read list of events")
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> AnyResult<()> {
        use schema::strava_events::dsl::*;

        diesel::delete(strava_events)
            .filter(id.eq_any(values))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> AnyResult<Vec<StravaUser>> {
        schema::strava_users::table
            .get_results::<StravaUser>(self)
            .await
            .context("get_stats: could not read users".to_string())
    }

    async fn stravauser_get_by_tbid(
        &mut self,
        id: UserId,
    ) -> Result<StravaUser, anyhow::Error> {
        schema::strava_users::table
            .filter(schema::strava_users::tendabike_id.eq(id))
            .get_result(self)
            .await
            .context(format!("User::get: user {} not registered", id))
    }

    async fn stravauser_get_by_stravaid(
        &mut self,
        id: StravaId,
    ) -> Result<Vec<StravaUser>, anyhow::Error> {
        schema::strava_users::table
            .find(id)
            .get_results::<StravaUser>(self)
            .await
            .context("failed to read stravauser")
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> Result<StravaUser, anyhow::Error> {
        diesel::insert_into(schema::strava_users::table)
            .values(&user)
            .get_result(self)
            .await
            .context("failed to insert user")
    }

    async fn stravauser_update_last_activity(
        &mut self,
        user: &StravaUser,
        time: i64,
    ) -> AnyResult<()> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(user.id))
            .set(last_activity.eq(time))
            .execute(self)
            .await
            .context("Could not update last_activity")?;
        Ok(())
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        access: &str,
        exp: i64,
        refresh: Option<&str>,
    ) -> AnyResult<StravaUser> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(stravaid))
            .set((
                access_token.eq(access),
                expires_at.eq(exp),
                refresh_token.eq(refresh.unwrap()),
            ))
            .get_result(self)
            .await
            .context("Could not store user")
    }

    /// return the open events and the disabled status for a user.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database connection fails.
    async fn strava_events_get_count_for_user(&mut self, user: &StravaUser) -> AnyResult<i64> {
        use schema::strava_events::dsl::*;

        strava_events
            .count()
            .filter(owner_id.eq(user.id))
            .first(self)
            .await
            .context("could not read strava events")
    }

    /// Disable the user data in the database by erasing the access token
    async fn stravauser_disable(&mut self, user: &StravaId) -> AnyResult<()> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(user))
            .set((expires_at.eq(0), access_token.eq("")))
            .execute(self)
            .await
            .context(format!("Could not disable record for user {}", user))?;
        Ok(())
    }

    async fn stravaid_lock(&mut self, user_id: &StravaId) -> AnyResult<bool> {
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

    async fn stravaid_unlock(&mut self, id: StravaId) -> AnyResult<usize> {
        sql_query(format!("SELECT pg_advisory_unlock({});", id))
            .execute(self)
            .await
            .context("Could not unlock stravaid")
    }
}
