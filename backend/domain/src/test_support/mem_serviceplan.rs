use super::*;
use crate::{PartId, ServicePlan, ServicePlanId, TbResult, UserId};

#[async_trait::async_trait]
impl ServicePlanStore for MemStore {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        todo!()
    }

    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        todo!()
    }

    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        todo!()
    }

    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        todo!()
    }

    async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize> {
        todo!()
    }

    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>> {
        todo!()
    }

    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>> {
        todo!()
    }
}
