use super::*;
use crate::{ActTypeId, Activity, ActivityId, PartId, TbResult, UserId};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl ActivityStore for MemStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        todo!()
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        todo!()
    }

    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        todo!()
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        todo!()
    }

    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> {
        todo!()
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        todo!()
    }

    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        todo!()
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        todo!()
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        todo!()
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        todo!()
    }
}
