use anyhow::Context;
use async_session::log::debug;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tb_domain::schema;
use crate::AsyncDieselConn;
use time::OffsetDateTime;

use tb_domain::{ActTypeId, Activity, ActivityId, AnyResult, NewActivity, PartId, Person, UserId};

#[async_session::async_trait]
impl tb_domain::ActivityStore for AsyncDieselConn {
    async fn activity_create(&mut self, act: &NewActivity) -> AnyResult<Activity> {
        diesel::insert_into(schema::activities::table)
            .values(act)
            .get_result(self)
            .await
            .context("Could not insert activity")
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> AnyResult<Activity> {
        schema::activities::table
            .find(aid)
            .for_update()
            .first::<Activity>(self)
            .await
            .context(format!("No activity id {}", aid))
    }

    async fn activity_update(&mut self, aid: ActivityId, act: &NewActivity) -> AnyResult<Activity> {
        use schema::activities;
        diesel::update(activities::table)
            .filter(activities::id.eq(aid))
            .set(act)
            .get_result::<Activity>(self)
            .await
            .context("Error updating activity")
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> AnyResult<usize> {
        use schema::activities::dsl::*;
        diesel::delete(activities.filter(id.eq(aid)))
            .execute(self)
            .await
            .context("Error deleting activity")
    }

    async fn activity_get_all_for_userid(&mut self, uid: UserId) -> AnyResult<Vec<Activity>> {
        use schema::activities::dsl::*;

        activities
            .filter(user_id.eq(uid))
            .order_by(start)
            .load::<Activity>(self)
            .await
            .context("error loading activities")
    }

    async fn activities_find_by_partid_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> AnyResult<Vec<Activity>> {
        use schema::activities::dsl::{activities, gear, start};

        activities
            .filter(gear.eq(Some(part)))
            .filter(start.ge(begin))
            .filter(start.lt(end))
            .load::<Activity>(self)
            .await
            .context("could not read activities")
    }

    async fn activity_get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> AnyResult<Activity> {
        use schema::activities::dsl::*;
        activities
            .filter(user_id.eq(uid))
            .filter(start.eq(rstart))
            .for_update()
            .get_result(self)
            .await
            .context(format!("could not read Activitiy at {}", rstart))
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: &dyn Person,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> AnyResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        diesel::update(activities)
            .filter(user_id.eq(user.get_id()))
            .filter(gear.is_null())
            .filter(what.eq_any(types))
            .set(gear.eq(partid))
            .get_results::<Activity>(self)
            .await
            .context("Error updating activities")
    }

    async fn part_reset_all(&mut self) -> AnyResult<usize> {
        use schema::parts::dsl::*;
        debug!("resetting all parts");
        diesel::update(parts)
            .set((
                time.eq(0),
                distance.eq(0),
                climb.eq(0),
                descend.eq(0),
                count.eq(0),
            ))
            .execute(self)
            .await
            .context("Could not reset parts")
    }

    async fn activity_get_really_all(&mut self) -> AnyResult<Vec<Activity>> {
        use schema::activities::dsl::*;
        activities
            .order_by(id)
            .get_results::<Activity>(self)
            .await
            .context("Could not get activities")
    }
}
