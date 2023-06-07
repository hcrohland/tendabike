use time::OffsetDateTime;

use crate::{ActTypeId, Activity, ActivityId, AnyResult, NewActivity, PartId, Person, UserId};

#[async_trait::async_trait]
pub trait ActivityStore {
    async fn activity_create(&mut self, act: &NewActivity) -> AnyResult<Activity>;

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> AnyResult<Activity>;

    async fn activity_update(&mut self, aid: ActivityId, act: &NewActivity) -> AnyResult<Activity>;

    async fn activity_delete(&mut self, aid: ActivityId) -> AnyResult<usize>;

    async fn activity_get_all_for_userid(&mut self, uid: UserId) -> AnyResult<Vec<Activity>>;

    async fn activities_find_by_partid_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> AnyResult<Vec<Activity>>;

    async fn activity_get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> AnyResult<Activity>;

    async fn activity_set_gear_if_null(
        &mut self,
        user: &dyn Person,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> AnyResult<Vec<Activity>>;

    async fn part_reset_all(&mut self) -> AnyResult<usize>;

    async fn activity_get_really_all(&mut self) -> AnyResult<Vec<Activity>>;
}
