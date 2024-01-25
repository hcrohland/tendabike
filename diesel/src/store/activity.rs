use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::{map_to_tb, AsyncDieselConn};
use tb_domain::{
    schema, ActTypeId, Activity, ActivityId, NewActivity, PartId, Person, TbResult, UserId,
};

#[async_session::async_trait]
impl tb_domain::ActivityStore for AsyncDieselConn {
    async fn activity_create(&mut self, act: &NewActivity) -> TbResult<Activity> {
        diesel::insert_into(schema::activities::table)
            .values(act)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Activity> {
        schema::activities::table
            .find(aid)
            .for_update()
            .first::<Activity>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_update(&mut self, aid: ActivityId, act: &NewActivity) -> TbResult<Activity> {
        use schema::activities;
        diesel::update(activities::table)
            .filter(activities::id.eq(aid))
            .set(act)
            .get_result::<Activity>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        use schema::activities::dsl::*;
        diesel::delete(activities.filter(id.eq(aid)))
            .execute(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::*;

        activities
            .filter(user_id.eq(uid))
            .order_by(start)
            .load::<Activity>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activities_find_by_partid_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::{activities, gear, start};

        activities
            .filter(gear.eq(Some(part)))
            .filter(start.ge(begin))
            .filter(start.lt(end))
            .load::<Activity>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        use schema::activities::dsl::*;
        activities
            .filter(user_id.eq(uid))
            .filter(start.eq(rstart))
            .for_update()
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: &dyn Person,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        diesel::update(activities)
            .filter(user_id.eq(user.get_id()))
            .filter(gear.is_null())
            .filter(what.eq_any(types))
            .set(gear.eq(partid))
            .get_results::<Activity>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        activities
            .order_by(id)
            .get_results::<Activity>(self)
            .await
            .map_err(map_to_tb)
    }
}
