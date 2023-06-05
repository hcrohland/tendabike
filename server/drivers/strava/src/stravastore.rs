use crate::event::Event;
use crate::AppConn;
use crate::StravaId;
use crate::StravaUser;
use anyhow::Context;
use anyhow::Result as AnyResult;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use domain::ActivityId;
use domain::NewPart;
use domain::PartId;
use domain::Person;
use domain::UserId;
use s_diesel::schema;

#[async_session::async_trait]
impl crate::Store for AppConn {
    async fn get_user_id_from_strava_id(&mut self, who: i32) -> AnyResult<i32> {
        use schema::strava_users::dsl::*;
        let user_id: i32 = strava_users
            .filter(tendabike_id.eq(who))
            .select(id)
            .first(self)
            .await?;
        Ok(user_id)
    }

    async fn store_stravaevent(&mut self, e: Event) -> AnyResult<()> {
        diesel::insert_into(schema::strava_events::table)
            .values(&e)
            .get_result::<Event>(&mut self)
            .await?;
        Ok(())
    }

    async fn get_tbid_for_strava_gear(&mut self, strava_id: &str) -> AnyResult<Option<PartId>> {
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

    async fn get_strava_name_for_gear_id(&mut self, gear: i32) -> Result<String, anyhow::Error> {
        use schema::strava_gears::dsl::*;
        let g: String = strava_gears
            .filter(tendabike_id.eq(gear))
            .select(id)
            .first(self)
            .await?;
        Ok(g)
    }

    async fn get_tbid_for_strava_activity(
        &mut self,
        strava_id: i64,
    ) -> AnyResult<Option<ActivityId>> {
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

    async fn insert_new_activity(
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

    async fn get_stravaid_for_tb_activity(&mut self, act: i32) -> Result<i64, anyhow::Error> {
        use schema::strava_activities::dsl::*;
        strava_activities
            .filter(tendabike_id.eq(act))
            .select(id)
            .first(self)
            .await
            .context("failed to get stravaid for activity")
    }

    async fn delete_strava_activity(&mut self, act_id: i64) -> Result<usize, anyhow::Error> {
        use schema::strava_activities::dsl::*;
        diesel::delete(strava_activities.find(act_id))
            .execute(self)
            .await
            .context("failed to delete strava activity")
    }

    async fn get_activityid_from_strava_activity(
        &mut self,
        act_id: i64,
    ) -> Result<Option<ActivityId>, anyhow::Error> {
        use schema::strava_activities::dsl::*;
        strava_activities
            .select(tendabike_id)
            .find(act_id)
            .first(self)
            .await
            .optional()
            .context("failed to get activity id")
    }

    async fn create_new_gear(
        &mut self,
        strava_id: String,
        part: NewPart,
        user: &dyn Person,
    ) -> Result<PartId, anyhow::Error> {
        self.transaction(|conn| {
            async {
                use schema::strava_gears::dsl::*;
                // maybe the gear was created by now?
                if let Some(gear) = conn.get_tbid_for_strava_gear(&strava_id).await? {
                    return Ok(gear);
                }

                let tbid = part.create(user, conn).await?.id;

                diesel::insert_into(strava_gears)
                    .values((
                        id.eq(strava_id),
                        tendabike_id.eq(tbid),
                        user_id.eq(user.get_id()),
                    ))
                    .execute(conn)
                    .await
                    .context("couldn't store gear")?;
                Ok(tbid)
            }
            .scope_boxed()
        })
        .await
    }

    async fn delete_strava_event(&mut self, event_id: Option<i32>) -> Result<(), anyhow::Error> {
        use schema::strava_events::dsl::*;
        diesel::delete(strava_events)
            .filter(id.eq(event_id))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn set_event_time(
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

    async fn get_next_event_for_stravauser(
        &mut self,
        user: &crate::StravaUser,
    ) -> AnyResult<Option<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(owner_id.eq_any(vec![0, user.id.into()]))
            .first::<Event>(self)
            .await
            .optional()
            .context("failed to get next event")
    }

    async fn get_all_later_events_for_object(
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

    async fn delete_events_by_vec_id(&mut self, values: Vec<Option<i32>>) -> AnyResult<()> {
        use schema::strava_events::dsl::*;

        diesel::delete(strava_events)
            .filter(id.eq_any(values))
            .execute(self)
            .await?;
        Ok(())
    }

    async fn get_all_stravausers(&mut self) -> AnyResult<Vec<StravaUser>> {
        schema::strava_users::table
            .get_results::<StravaUser>(self)
            .await
            .context("get_stats: could not read users".to_string())
    }

    async fn read_stravauser_for_userid(
        &mut self,
        id: UserId,
    ) -> Result<StravaUser, anyhow::Error> {
        schema::strava_users::table
            .filter(schema::strava_users::tendabike_id.eq(id))
            .get_result(self)
            .await
            .context(format!("User::get: user {} not registered", id))
    }

    async fn read_stravauser_for_stravaid(
        &mut self,
        id: StravaId,
    ) -> Result<Vec<StravaUser>, anyhow::Error> {
        schema::strava_users::table
            .find(id)
            .get_results::<StravaUser>(self)
            .await
            .context("failed to read stravauser")
    }

    async fn insert_stravauser(&mut self, user: StravaUser) -> Result<StravaUser, anyhow::Error> {
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
    async fn get_count_of_events_for_user(&mut self, user: &StravaUser) -> AnyResult<i64> {
        use schema::strava_events::dsl::*;

        strava_events
            .count()
            .filter(owner_id.eq(user.id))
            .first(self)
            .await
            .context("could not read strava events")
    }

    /// Disable the user data in the database by erasing the access token
    async fn disable_stravauser(&mut self, user: &StravaId) -> AnyResult<()> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(user))
            .set((expires_at.eq(0), access_token.eq("")))
            .execute(self)
            .await
            .context(format!("Could not disable record for user {}", user))?;
        Ok(())
    }
}