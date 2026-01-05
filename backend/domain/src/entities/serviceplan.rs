use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::*;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServicePlanId(Uuid);

NewtypeDisplay! { () pub struct ServicePlanId(); }
NewtypeFrom! { () pub struct ServicePlanId(Uuid); }

impl ServicePlanId {
    pub(crate) fn new() -> Self {
        Uuid::now_v7().into()
    }

    async fn get(self, store: &mut impl ServicePlanStore) -> TbResult<ServicePlan> {
        store.get(self).await
    }

    pub async fn delete(
        self,
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<Vec<Service>> {
        let plan = self.get(store).await?;
        plan.checkuser(user, store).await?;

        let res = Service::reset_plan(self, store).await?;

        // delete service
        ServicePlanStore::delete(store, self).await?;
        Ok(res)
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
}

impl ServicePlan {
    async fn checkuser(
        &self,
        user: &dyn Session,
        store: &mut (impl ServicePlanStore + PartStore),
    ) -> TbResult<()> {
        if let Some(part) = self.part {
            part.checkuser(user, store).await?;
        } else if self.uid != Some(user.get_id()) {
            return Err(crate::Error::BadRequest(format!(
                "user mismatch {} != {:?}",
                user.get_id(),
                self.uid
            )));
        }
        Ok(())
    }

    pub async fn create(
        mut self,
        user: &dyn Session,
        store: &mut (impl ServicePlanStore + PartStore),
    ) -> TbResult<Self> {
        self.id = ServicePlanId::new();
        self.uid = Some(user.get_id());
        store.create(self).await
    }

    pub async fn update(
        mut self,
        user: &dyn Session,
        store: &mut (impl ServicePlanStore + PartStore),
    ) -> TbResult<ServicePlan> {
        let plan = self.id.get(store).await?;
        plan.checkuser(user, store).await?;
        // You cannot change these
        self.part = plan.part;
        self.what = plan.what;
        self.hook = plan.hook;
        self.uid = plan.uid;
        store.update(self).await
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
}
