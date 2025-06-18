use anyhow::Context;
use diesel::{prelude::*, sql_query};
use diesel_async::RunQueryDsl;
use time::{OffsetDateTime, UtcOffset};

use crate::{AsyncDieselConn, into_domain, vec_into};
use tb_domain::{ActTypeId, Activity, ActivityId, NewActivity, PartId, Person, TbResult, UserId};
mod schema {
    use diesel::prelude::*;

    table! {
        activities (id) {
            id -> Nullable<Int4>,
            user_id -> Int4,
            what -> Int4,
            name -> Text,
            start -> Timestamptz,
            duration -> Int4,
            time -> Nullable<Int4>,
            distance -> Nullable<Int4>,
            climb -> Nullable<Int4>,
            descend -> Nullable<Int4>,
            energy -> Nullable<Int4>,
            gear -> Nullable<Int4>,
            utc_offset -> Int4,
        }
    }
}
#[derive(
    Debug, Clone, Insertable, Identifiable, Queryable, QueryableByName, AsChangeset, PartialEq,
)]
#[diesel(table_name = schema::activities)]
struct DbActivity {
    /// The primary key
    id: Option<i32>,
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
}

impl From<&NewActivity> for DbActivity {
    fn from(v: &NewActivity) -> Self {
        let utc_offset = v.start.offset().whole_seconds();
        DbActivity {
            id: None,
            user_id: v.user_id.into(),
            what: v.what.into(),
            name: v.name.clone(),
            start: v.start,
            duration: v.duration,
            time: v.time,
            distance: v.distance,
            climb: v.climb,
            descend: v.descend,
            energy: v.energy,
            gear: v.gear.map(Into::into),
            utc_offset,
        }
    }
}

impl TryFrom<DbActivity> for Activity {
    type Error = tb_domain::Error;

    fn try_from(v: DbActivity) -> Result<Self, Self::Error> {
        let id = v.id.expect("DbActivity with Null id");
        let offset = UtcOffset::from_whole_seconds(v.utc_offset).context("Utc Offset invalid")?;
        let start = v.start.to_offset(offset);

        Ok(Activity {
            id: id.into(),
            user_id: v.user_id.into(),
            what: v.what.into(),
            name: v.name.clone(),
            start,
            duration: v.duration,
            time: v.time,
            distance: v.distance,
            climb: v.climb,
            descend: v.descend,
            energy: v.energy,
            gear: v.gear.map(Into::into),
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
    async fn activity_create(&mut self, act: &NewActivity) -> TbResult<Activity> {
        diesel::insert_into(schema::activities::table)
            .values(DbActivity::from(act))
            .get_result::<DbActivity>(self)
            .await
            .map_err(into_domain)?
            .try_into()
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Activity> {
        schema::activities::table
            .filter(schema::activities::id.eq(i32::from(aid)))
            .for_update()
            .first::<DbActivity>(self)
            .await
            .map_err(into_domain)?
            .try_into()
    }

    async fn activity_update(&mut self, aid: ActivityId, act: &NewActivity) -> TbResult<Activity> {
        diesel::update(schema::activities::table)
            .filter(schema::activities::id.eq(i32::from(aid)))
            .set(DbActivity::from(act))
            .get_result::<DbActivity>(self)
            .await
            .map_err(into_domain)?
            .try_into()
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        use schema::activities::dsl::*;
        diesel::delete(activities.filter(id.eq(i32::from(aid))))
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
}
