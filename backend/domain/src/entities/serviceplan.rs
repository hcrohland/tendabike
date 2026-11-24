use std::collections::HashMap;

use crate::{
    Error, IdKeyed, PartId, PartStore, PartTypeId, Service, ServicePlanStore, ServiceStore,
    Session, ShopStore, Summary, TbResult, UserId,
};
#[cfg(test)]
use crate::{Part, ServiceId, Shop, ShopId, UsageId};
use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub struct ServicePlanId(Uuid);

impl ServicePlanId {
    pub(crate) fn new() -> Self {
        Uuid::now_v7().into()
    }

    async fn get(self, store: &mut impl ServicePlanStore) -> TbResult<ServicePlan> {
        store.serviceplan_get(self).await
    }

    pub(crate) async fn delete(
        self,
        user: &dyn Session,
        store: &mut (impl PartStore + ServicePlanStore + ServiceStore + ShopStore),
    ) -> TbResult<Vec<Service>> {
        let plan = self.get(store).await?;
        plan.checkuser(user, store).await?;

        let owner = match plan.part {
            Some(part) => part.read(store).await?.owner,
            None => plan.uid.unwrap_or(user.user_id()),
        };

        let res = Service::unlink_plan(self, owner, store).await?;

        // delete service plan
        ServicePlanStore::serviceplan_delete(store, self).await?;
        Ok(res)
    }

    /// Snoozes this plan for `part` by `fraction` (issue #971): the
    /// fraction is added to the plan's accumulated snooze fraction for
    /// the part, so snoozes stack and the fraction is unbounded above.
    /// The floor is 0; a fraction of 0 is a no-op (no write, an empty
    /// `Summary`). Snoozing records no work: no service is created or
    /// modified. Returns the `Summary` carrying the updated plan.
    pub(crate) async fn snooze(
        self,
        part: PartId,
        fraction: f64,
        user: &dyn Session,
        store: &mut (impl PartStore + ServicePlanStore + ShopStore),
    ) -> TbResult<Summary> {
        // Non-finite fractions (NaN, ±inf) are rejected as well: the
        // store cannot serialize them.
        if !fraction.is_finite() || fraction < 0.0 {
            return Err(Error::BadRequest(format!(
                "snooze fraction must be a finite non-negative number, got {fraction}"
            )));
        }
        let mut plan = self.get(store).await?;
        plan.checkuser(user, store).await?;
        if fraction == 0.0 {
            return Ok(Summary::default());
        }
        let accumulated = plan.snoozes.get(&part).copied().unwrap_or(0.0) + fraction;
        plan.snoozes.insert(part, accumulated);
        let plan = store.serviceplan_update(plan).await?;
        let mut summary = Summary::default();
        summary.plans.upsert(plan);
        Ok(summary)
    }

    /// Clears this plan's accumulated snooze for `part` (issue #1011): a
    /// real service ends the postponement, so the part's due-ness returns
    /// to the plain limit minus usage. Returns the updated plan, or `None`
    /// when there was no accumulated snooze for the part (no write, mirroring
    /// the snooze op's no-op rule). Snoozes stay a way to postpone due-ness
    /// without work: only a real service (or a plan-limit edit) ends them.
    pub(crate) async fn clear_snooze(
        self,
        part: PartId,
        store: &mut impl ServicePlanStore,
    ) -> TbResult<Option<ServicePlan>> {
        let mut plan = self.get(store).await?;
        if plan.snoozes.remove(&part).is_some() {
            Ok(Some(store.serviceplan_update(plan).await?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicePlan {
    #[serde(default = "ServicePlanId::new")]
    pub id: ServicePlanId,
    /// the gear or part involved
    /// if hook is None the plan is for a specific part
    /// if it's Some(hook) it is a generic plan for that hook
    pub part: Option<PartId>,
    /// This is only really used for generic plans
    /// for a specific part it is set to the PartType of the part
    pub what: PartTypeId,
    /// where it is attached
    pub hook: Option<PartTypeId>,
    pub name: String,
    /// Time until service
    pub days: Option<i32>,
    /// Usage time
    pub hours: Option<i32>,
    /// Usage distance
    pub km: Option<i32>,
    /// Overall climbing
    pub climb: Option<i32>,
    /// Overall descending
    pub descend: Option<i32>,
    /// number of activities
    pub rides: Option<i32>,
    /// User for generic plans
    pub uid: Option<UserId>,
    /// Energy expended
    #[serde(rename = "kJ")]
    pub energy: Option<i32>,
    /// The accumulated snooze fraction per part (issue #971): snoozing a
    /// (part, plan) adds the snoozed fraction to that part's entry,
    /// extending the plan's effective limits for it. The plan owns the
    /// state; a part holds none of it. Editing the plan's limits resets
    /// this map to empty.
    #[serde(default)]
    pub snoozes: HashMap<PartId, f64>,
}

impl IdKeyed for ServicePlan {
    type Key = ServicePlanId;
    fn key(&self) -> Self::Key {
        self.id
    }
}

impl ServicePlan {
    /// Deletes a specific set of service plans. The user-deletion cascade
    /// (ADR-0012) uses this: the caller already owns these plans.
    pub(crate) async fn delete_many(
        plans: &[ServicePlan],
        store: &mut impl ServicePlanStore,
    ) -> TbResult<usize> {
        store.serviceplans_delete(plans).await
    }

    async fn checkuser(
        &self,
        user: &dyn Session,
        store: &mut (impl PartStore + ShopStore),
    ) -> TbResult<()> {
        if let Some(part) = self.part {
            part.checkuser(user, store).await?;
        } else if self.uid != Some(user.user_id()) {
            return Err(crate::Error::BadRequest(format!(
                "user mismatch {} != {:?}",
                user.user_id(),
                self.uid
            )));
        }
        Ok(())
    }

    pub(crate) async fn create(
        mut self,
        user: &dyn Session,
        store: &mut (impl PartStore + ServicePlanStore),
    ) -> TbResult<Self> {
        self.id = ServicePlanId::new();
        self.uid = match self.part {
            Some(_) => None,
            None => Some(user.user_id()),
        };
        // snoozes are server state: a fresh plan has none, whatever the
        // client sent.
        self.snoozes = HashMap::new();
        store.serviceplan_create(self).await
    }

    pub(crate) async fn update(
        mut self,
        user: &dyn Session,
        store: &mut (impl PartStore + ServicePlanStore + ShopStore),
    ) -> TbResult<ServicePlan> {
        let plan = self.id.get(store).await?;
        plan.checkuser(user, store).await?;
        // You cannot change these
        self.part = plan.part;
        self.what = plan.what;
        self.hook = plan.hook;
        self.uid = plan.uid;
        // editing the plan's limits resets all of its snoozes (issue #971)
        self.snoozes = HashMap::new();
        store.serviceplan_update(self).await
    }

    pub(crate) async fn for_part(
        part: PartId,
        store: &mut impl ServicePlanStore,
    ) -> TbResult<Vec<Self>> {
        store.by_part(part).await
    }

    pub(crate) async fn for_user(
        uid: &UserId,
        store: &mut impl ServicePlanStore,
    ) -> TbResult<Vec<Self>> {
        store.by_user(*uid).await
    }

    /// Removes `part`'s entry from the snooze map of every plan that carries
    /// it (issue #1013): the part-deletion cascade. A part with a specific
    /// plan cannot be deleted while the plan is active, so only generic plans
    /// carry an entry. Returns the updated plans (none when no plan carried an
    /// entry — no writes).
    pub(crate) async fn clear_snoozes_for_part(
        part: PartId,
        store: &mut impl ServicePlanStore,
    ) -> TbResult<Vec<Self>> {
        let mut updated = Vec::new();
        for mut plan in store.plans_snoozing(part).await? {
            plan.snoozes.remove(&part);
            updated.push(store.serviceplan_update(plan).await?);
        }
        Ok(updated)
    }
}

#[cfg(test)]
mod serviceplan_tests;
