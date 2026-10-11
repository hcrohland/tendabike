use async_trait::async_trait;
use tb_domain::{PartId, ServicePlan, ServicePlanId, ServicePlanStore, TbResult, UserId};

use super::TestStore;

#[async_trait]
impl ServicePlanStore for TestStore {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        d.service_plans.insert(plan.id, plan.clone());
        Ok(plan)
    }

    async fn get(&mut self, id: ServicePlanId) -> TbResult<ServicePlan> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        d.service_plans
            .get(&id)
            .cloned()
            .ok_or(tb_domain::Error::NotFound(format!(
                "ServicePlan {} not found",
                id
            )))
    }

    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        match d.service_plans.get_mut(&plan.id) {
            Some(p) => {
                *p = plan.clone();
                Ok(plan)
            }
            None => Err(tb_domain::Error::NotFound(format!(
                "ServicePlan {} not found",
                plan.id
            ))),
        }
    }

    async fn delete(&mut self, id: ServicePlanId) -> TbResult<usize> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        match d.service_plans.remove(&id) {
            Some(_) => Ok(1),
            None => Ok(0),
        }
    }

    async fn serviceplans_delete(&mut self, plans: &[ServicePlan]) -> TbResult<usize> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        let mut count = 0;
        for p in plans {
            if d.service_plans.remove(&p.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        Ok(d.service_plans
            .values()
            .filter(|p| p.part == Some(part))
            .cloned()
            .collect())
    }

    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        Ok(d.service_plans
            .values()
            .filter(|p| p.uid == Some(uid))
            .cloned()
            .collect())
    }
}
