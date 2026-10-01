use super::*;
use crate::{PartId, ServicePlan, ServicePlanId, TbResult, UserId};

#[async_trait::async_trait]
impl ServicePlanStore for MemStore {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let d = self.state_mut();
        d.service_plans.insert(plan.id, plan.clone());
        Ok(plan)
    }

    async fn get(&mut self, id: ServicePlanId) -> TbResult<ServicePlan> {
        let d = self.state();
        d.service_plans
            .get(&id)
            .cloned()
            .ok_or(crate::Error::NotFound(format!(
                "ServicePlan {} not found",
                id
            )))
    }

    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let d = self.state_mut();
        match d.service_plans.get_mut(&plan.id) {
            Some(p) => {
                *p = plan.clone();
                Ok(plan)
            }
            None => Err(crate::Error::NotFound(format!(
                "ServicePlan {} not found",
                plan.id
            ))),
        }
    }

    async fn delete(&mut self, id: ServicePlanId) -> TbResult<usize> {
        let d = self.state_mut();
        match d.service_plans.remove(&id) {
            Some(_) => Ok(1),
            None => Ok(0),
        }
    }

    async fn serviceplans_delete(&mut self, plans: &[ServicePlan]) -> TbResult<usize> {
        let d = self.state_mut();
        let mut count = 0;
        for p in plans {
            if d.service_plans.remove(&p.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>> {
        let d = self.state();
        Ok(d.service_plans
            .values()
            .filter(|p| p.part == Some(part))
            .cloned()
            .collect())
    }

    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>> {
        let d = self.state();
        Ok(d.service_plans
            .values()
            .filter(|p| p.uid == Some(uid))
            .cloned()
            .collect())
    }
}
