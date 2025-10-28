use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use oauth2::RefreshToken;

use crate::{AsyncDieselConn, into_domain, option_into, vec_into};
use tb_domain::{TbResult, UserId};
use tb_strava::{StravaId, StravaUser, event::Event};

mod schema;
#[derive(Clone, Queryable, Insertable, Identifiable, Debug, Default)]
#[diesel(table_name = schema::strava_users)]
pub struct DbStravaUser {
    id: i32,
    tendabike_id: i32,
    refresh_token: Option<String>,
}

impl From<StravaUser> for DbStravaUser {
    fn from(value: StravaUser) -> Self {
        let StravaUser {
            id,
            tendabike_id,
            refresh_token,
        } = value;
        Self {
            id: id.into(),
            tendabike_id: tendabike_id.into(),
            refresh_token: refresh_token.map(RefreshToken::into_secret),
        }
    }
}
impl From<DbStravaUser> for StravaUser {
    fn from(value: DbStravaUser) -> Self {
        let DbStravaUser {
            id,
            tendabike_id,
            refresh_token,
        } = value;
        Self {
            id: id.into(),
            tendabike_id: tendabike_id.into(),
            refresh_token: refresh_token.map(RefreshToken::new),
        }
    }
}

#[derive(Debug, Default, Queryable, Insertable)]
#[diesel(table_name = schema::strava_events)]
pub struct DbEvent {
    id: Option<i32>,
    pub object_type: String,
    pub object_id: i64,
    pub aspect_type: String,
    updates: String,
    owner_id: i32,
    subscription_id: i32,
    pub event_time: i64,
}

impl From<Event> for DbEvent {
    fn from(value: Event) -> Self {
        let Event {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id,
            subscription_id,
            event_time,
        } = value;
        Self {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id: owner_id.into(),
            subscription_id,
            event_time,
        }
    }
}

impl From<DbEvent> for Event {
    fn from(value: DbEvent) -> Self {
        let DbEvent {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id,
            subscription_id,
            event_time,
        } = value;
        Self {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id: owner_id.into(),
            subscription_id,
            event_time,
        }
    }
}

#[async_session::async_trait]
impl tb_strava::StravaStore for AsyncDieselConn {
    async fn stravaid_get_user_id(&mut self, who: i32) -> TbResult<i32> {
        use schema::strava_users::dsl::*;
        strava_users
            .filter(tendabike_id.eq(who))
            .select(id)
            .first(self)
            .await
            .map_err(into_domain)
    }

    async fn stravaevent_store(&mut self, e: Event) -> TbResult<()> {
        let e: DbEvent = e.into();
        diesel::insert_into(schema::strava_events::table)
            .values(&e)
            .get_result::<DbEvent>(&mut self)
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

    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
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
        user_id: StravaId,
    ) -> TbResult<Option<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(owner_id.eq_any(vec![0, user_id.into()]))
            .order(event_time.asc())
            .first::<DbEvent>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(option_into)
    }

    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        use schema::strava_events::dsl::*;
        strava_events
            .filter(object_id.eq(obj_id))
            .filter(owner_id.eq(i32::from(oid)))
            .order(event_time.asc())
            .get_results::<DbEvent>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
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
            .get_results::<DbStravaUser>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        schema::strava_users::table
            .filter(schema::strava_users::tendabike_id.eq(i32::from(id)))
            .get_result::<DbStravaUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        schema::strava_users::table
            .find(i32::from(*id))
            .first::<DbStravaUser>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(option_into)
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        diesel::insert_into(schema::strava_users::table)
            .values(DbStravaUser::from(user))
            .get_result::<DbStravaUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> TbResult<StravaUser> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(i32::from(stravaid)))
            .set((refresh_token.eq(refresh),))
            .get_result::<DbStravaUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
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
            .filter(owner_id.eq(i32::from(*user)))
            .first(self)
            .await
            .map_err(into_domain)
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        use schema::strava_events::dsl::*;

        diesel::delete(strava_events.filter(owner_id.eq(i32::from(*user))))
            .execute(self)
            .await
            .map_err(into_domain)
    }

    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        use schema::strava_users::dsl::*;

        diesel::delete(strava_users.filter(tendabike_id.eq(i32::from(user))))
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
