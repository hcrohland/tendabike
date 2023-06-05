use time::OffsetDateTime;
use crate::{AnyResult, Part, PartId, PartTypeId, Person, Usage, UserId};
#[async_trait::async_trait]
pub trait PartStore {
    async fn partid_get_part(&mut self, pid: PartId) -> AnyResult<Part>;

    async fn partid_get_name(&mut self, pid: PartId) -> AnyResult<String>;

    async fn partid_get_type(&mut self, pid: PartId) -> AnyResult<PartTypeId>;

    async fn partid_get_ownerid(&mut self, pid: PartId, user: &dyn Person)
        -> AnyResult<UserId>;

    async fn partid_apply_usage(
        &mut self,
        pid: PartId,
        usage: &Usage,
        start: OffsetDateTime,
    ) -> AnyResult<Part>;

    async fn part_get_all_for_userid(&mut self, uid: UserId) -> AnyResult<Vec<Part>>;

    async fn parts_reset_all_usages(&mut self, uid: UserId) -> AnyResult<Vec<Part>>;

    async fn create_part(
        &mut self,
        newpart: crate::NewPart,
        createtime: OffsetDateTime,
    ) -> AnyResult<Part>;

    async fn part_change(&mut self, part: crate::ChangePart) -> AnyResult<Part>;
}
