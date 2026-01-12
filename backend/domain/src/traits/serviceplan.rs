use crate::{PartId, ServicePlan, ServicePlanId, TbResult, UserId};

#[async_trait::async_trait]
/// A trait representing a store for `Usage` objects.
pub trait ServicePlanStore {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan>;
    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize>;

    /// Deletes an array of serviceplans
    ///
    /// # Arguments
    ///
    /// * `serviceplans` - An Vector of serviceplans to delete
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the number of deleted serviceplans or an error if the operation fails.
    async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize>;

    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>>;
    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>>;
}
