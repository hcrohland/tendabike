use crate::{PartId, ServicePlan, ServicePlanId, TbResult};

#[async_trait::async_trait]
/// A trait representing a store for `Usage` objects.
pub trait ServicePlanStore {
    async fn create(&mut self, plan: &ServicePlan) -> TbResult<ServicePlan>;
    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan>;
    async fn update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
    async fn plans_by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>>;
}
