use super::*;
use crate::{PartId, ServicePlan, ServicePlanId, TbResult, UserId};

#[async_trait::async_trait]
impl ServicePlanStore for MemStore {
    async fn create(&mut self, _plan: ServicePlan) -> TbResult<ServicePlan> {
        todo!()
    }

    async fn get(&mut self, _plan: ServicePlanId) -> TbResult<ServicePlan> {
        todo!()
    }

    async fn plan_update(&mut self, _plan: ServicePlan) -> TbResult<ServicePlan> {
        todo!()
    }

    async fn delete(&mut self, _plan: ServicePlanId) -> TbResult<usize> {
        todo!()
    }

    async fn serviceplans_delete(&mut self, _serviceplans: &[ServicePlan]) -> TbResult<usize> {
        todo!()
    }

    async fn by_part(&mut self, _part: PartId) -> TbResult<Vec<ServicePlan>> {
        Ok(self.service_plans.values().cloned().collect())
    }

    async fn by_user(&mut self, _uid: UserId) -> TbResult<Vec<ServicePlan>> {
        todo!()
    }
}
