use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::*;
use schema::service_plans;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    // Queryable,
    // Identifiable,
    // Insertable,
    // AsChangeset,
)]
#[repr(i64)]
pub enum ServiceAlert {
    NoService,
    Days(i32),
    Time(i32),
    Distance(i32),
    Climb(i32),
    Descend(i32),
    Count(i32),
}

#[derive(
    DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
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

    pub async fn delete(self, user: &dyn Person, store: &mut impl Store) -> TbResult<Vec<Service>> {
        let plan = self.get(store).await?;
        plan.part.checkuser(user, store).await?;

        let res = Service::reset_plan(self, store).await?;

        // delete service
        ServicePlanStore::delete(store, self).await?;
        Ok(res)
    }
    /* async fn check(
        self,
        last_service: OffsetDateTime,
        usage: &Usage,
        store: &mut impl ServicePlanStore,
    ) -> TbResult<ServiceAlert> {
        use time::{Duration, OffsetDateTime};
        use ServiceAlert::*;
        let plan = self.get(store).await?;
        if let Some(plan) = plan.days {
            if Duration::days((plan).into()) <= OffsetDateTime::now_utc() - last_service {
                return Ok(Time(plan));
            };
        }
        if let Some(plan) = plan.time {
            if plan <= usage.time {
                return Ok(Time(plan));
            };
        }
        if let Some(plan) = plan.climb {
            if plan <= usage.climb {
                return Ok(Climb(plan));
            };
        }
        if let Some(plan) = plan.descend {
            if plan <= usage.descend {
                return Ok(Descend(plan));
            };
        }
        if let Some(plan) = plan.distance {
            if plan <= usage.distance {
                return Ok(Distance(plan));
            };
        }
        if let Some(plan) = plan.count {
            if plan <= usage.count {
                return Ok(Count(plan));
            };
        }
        Ok(NoService)
    } */
}

///
///
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
)]
pub struct ServicePlan {
    #[serde(default = "ServicePlanId::new")]
    pub id: ServicePlanId,
    /// the gear or part involved
    /// if hook is None the plan is for a specific part
    /// if it's Some(hook) it is a generic plan for that hook
    part: PartId,
    /// This is only really used for generic plans
    /// for a specific part it is set to the PartType of the part
    what: PartTypeId,
    /// where it is attached
    hook: Option<PartTypeId>,
    name: String,
    /// Time until service
    pub days: Option<i32>,
    /// Usage time
    pub time: Option<i32>,
    /// Usage distance
    pub distance: Option<i32>,
    /// Overall climbing
    pub climb: Option<i32>,
    /// Overall descending
    pub descend: Option<i32>,
    /// number of activities
    pub count: Option<i32>,
}

impl ServicePlan {
    pub async fn create(
        mut self,
        user: &dyn Person,
        store: &mut (impl ServicePlanStore + PartStore),
    ) -> TbResult<Self> {
        self.part.checkuser(user, store).await?;
        self.id = ServicePlanId::new();
        store.create(self).await
    }

    pub async fn update(
        mut self,
        user: &dyn Person,
        store: &mut (impl ServicePlanStore + PartStore),
    ) -> TbResult<ServicePlan> {
        let plan = self.id.get(store).await?;
        plan.part.checkuser(user, store).await?;
        // You cannot change these
        self.part = plan.part;
        self.what = plan.what;
        self.hook = plan.hook;
        store.update(self).await
    }

    pub(crate) async fn for_part(
        part: PartId,
        store: &mut impl ServicePlanStore,
    ) -> TbResult<Vec<Self>> {
        store.by_part(part).await
    }
}
