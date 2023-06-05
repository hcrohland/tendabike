use crate::{PartType, PartTypeId, AnyResult, ActTypeId, ActivityType};

#[async_trait::async_trait]
pub trait Store {
    async fn get_all_parttypes_ordered(&mut self) -> Vec<PartType>;

    async fn get_parttype_by_id(&mut self, pid: PartTypeId) -> AnyResult<PartType>;

    async fn get_activity_types_by_parttypeid(&mut self, tid: &PartTypeId) -> AnyResult<Vec<ActTypeId>>;

    async fn get_all_activitytypes_order(&mut self) -> Vec<ActivityType>;
}