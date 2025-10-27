use super::schema;
use crate::{AsyncDieselConn, into_domain, vec_into};
use anyhow::Context;
use diesel::expression_methods::ExpressionMethods;

use diesel::{prelude::*, sql_query};
use diesel_async::RunQueryDsl;
use tb_domain::{ActTypeId, Activity, ActivityId, PartId, Person, TbResult, UserId};
use time::{OffsetDateTime, UtcOffset};

#[derive(
    Debug, Clone, Insertable, Identifiable, Queryable, QueryableByName, AsChangeset, PartialEq,
)]
#[diesel(table_name = schema::activities)]
struct DbActivity {
    /// The athlete
    user_id: i32,
    /// The activity type
    what: i32,
    /// This name of the activity.
    name: String,
    /// Start time
    start: OffsetDateTime,
    /// End time
    duration: i32,
    /// activity time
    time: Option<i32>,
    /// Covered distance
    distance: Option<i32>,
    /// Total climbing
    climb: Option<i32>,
    /// Total descending
    descend: Option<i32>,
    /// average energy output
    energy: Option<i32>,
    /// Which gear did she use?
    gear: Option<i32>,
    /// utc offset since timstamptz does not store the timezone
    utc_offset: i32,
    /// The primary key
    id: i64,
}

impl From<Activity> for DbActivity {
    fn from(v: Activity) -> Self {
        let Activity {
            id,
            user_id,
            what,
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear,
        } = v;
        let utc_offset = start.offset().whole_seconds();
        DbActivity {
            id: id.into(),
            user_id: user_id.into(),
            what: what.into(),
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear: gear.map(Into::into),
            utc_offset,
        }
    }
}

impl TryFrom<DbActivity> for Activity {
    type Error = tb_domain::Error;

    fn try_from(v: DbActivity) -> Result<Self, Self::Error> {
        let DbActivity {
            id,
            user_id,
            what,
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear,
            utc_offset,
        } = v;
        let utc_offset = ((utc_offset + 900) / 1800) * 1800; //round it to 1800s
        let offset = UtcOffset::from_whole_seconds(utc_offset).context("Utc Offset invalid")?;
        let start = start.to_offset(offset);

        Ok(Activity {
            id: id.into(),
            user_id: user_id.into(),
            what: what.into(),
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear: gear.map(Into::into),
        })
    }
}

fn vec_tryinto(db: Result<Vec<DbActivity>, diesel::result::Error>) -> TbResult<Vec<Activity>> {
    db.map_err(into_domain)?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
}

#[async_session::async_trait]
impl tb_domain::ActivityStore for AsyncDieselConn {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        let values = DbActivity::from(act);
        diesel::insert_into(schema::activities::table)
            .values(&values)
            .get_result::<DbActivity>(self)
            .await
            .map_err(into_domain)?
            .try_into()
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        schema::activities::table
            .find(i64::from(aid))
            .for_update()
            .first::<DbActivity>(self)
            .await
            .optional()
            .map_err(into_domain)?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        let act = DbActivity::from(act);
        diesel::update(&act)
            .set(&act)
            .get_result::<DbActivity>(self)
            .await
            .map_err(into_domain)?
            .try_into()
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        use schema::activities::dsl::*;
        diesel::delete(activities.find(i64::from(aid)))
            .execute(self)
            .await
            .map_err(into_domain)
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::*;

        vec_tryinto(
            activities
                .filter(user_id.eq(i32::from(*uid)))
                .order_by(start)
                .load::<DbActivity>(self)
                .await,
        )
    }

    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::{activities, gear, start};

        vec_tryinto(
            activities
                .filter(gear.eq(Some(i32::from(part))))
                .filter(start.ge(begin))
                .filter(start.lt(end))
                .load::<DbActivity>(self)
                .await,
        )
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        use diesel::sql_types;
        let query = sql_query(
            "SELECT * FROM activities WHERE user_id = $1 AND date_trunc('minute',start) + make_interval(0,0,0,0,0,0,utc_offset) = date_trunc('minute',$2) FOR UPDATE",
        )
        .bind::<sql_types::Int4, _>(i32::from(uid))
        .bind::<sql_types::Timestamptz, _>(rstart);
        query
            .get_result::<DbActivity>(self)
            .await
            .map_err(into_domain)?
            .try_into()
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: &dyn Person,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        let types: Vec<i32> = vec_into(types);
        vec_tryinto(
            diesel::update(activities)
                .filter(user_id.eq(i32::from(user.get_id())))
                .filter(gear.is_null())
                .filter(what.eq_any(types))
                .set(gear.eq(i32::from(*partid)))
                .get_results::<DbActivity>(self)
                .await,
        )
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        vec_tryinto(
            activities
                .order_by(id)
                .get_results::<DbActivity>(self)
                .await,
        )
    }

    async fn activities_delete(&mut self, acts: &[Activity]) -> TbResult<usize> {
        use schema::activities::dsl::*;
        let acts: Vec<_> = acts.iter().map(|s| i64::from(s.id)).collect();

        diesel::delete(activities.filter(id.eq_any(acts)))
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
