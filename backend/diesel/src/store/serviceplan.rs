use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use tb_domain::{schema, PartId, ServicePlan, ServicePlanId, TbResult, UserId};

use crate::{map_to_tb, AsyncDieselConn};
use schema::service_plans::dsl::*;

#[async_session::async_trait]
impl tb_domain::ServicePlanStore for AsyncDieselConn {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        diesel::insert_into(service_plans)
            .values(plan)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        service_plans
            .find(plan)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        diesel::update(service_plans.find(plan.id))
            .set(plan)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        diesel::delete(service_plans.find(plan))
            .execute(self)
            .await
            .map_err(map_to_tb)
    }

    async fn by_part(&mut self, part_id: PartId) -> TbResult<Vec<ServicePlan>> {
        service_plans
            .filter(part.eq(part_id))
            .get_results(self)
            .await
            .map_err(map_to_tb)
    }

    async fn by_user(&mut self, user_id: &UserId) -> TbResult<Vec<ServicePlan>> {
        service_plans
            .filter(uid.eq(user_id))
            .get_results(self)
            .await
            .map_err(map_to_tb)
    }
}
