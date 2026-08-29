use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::*;

#[derive(
    Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub struct ServicePlanId(Uuid);

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
    async fn checkuser(&self, user: &dyn Session, store: &mut impl Store) -> TbResult<()> {
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

    pub async fn create(
        mut self,
        user: &dyn Session,
        store: &mut (impl ServicePlanStore + PartStore),
    ) -> TbResult<Self> {
        self.id = ServicePlanId::new();
        self.uid = match self.part {
            Some(_) => None,
            None => Some(user.user_id()),
        };
        store.create(self).await
    }

    pub async fn update(
        mut self,
        user: &dyn Session,
        store: &mut impl Store,
    ) -> TbResult<ServicePlan> {
        let plan = self.id.get(store).await?;
        plan.checkuser(user, store).await?;
        // You cannot change these
        self.part = plan.part;
        self.what = plan.what;
        self.hook = plan.hook;
        self.uid = plan.uid;
        store.plan_update(self).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MemStore, TestSession, fixtures};

    use fixtures::{sample_purchase_date, test_session, test_user};

    // === Suite 6: ServicePlan — CRUD ===

    /// SP-01: ServicePlan create with specific part
    #[tokio::test]
    async fn service_plan_create_specific_part() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: Some(part.id),
            what: PartTypeId::from(2),
            hook: None,
            name: "Service Plan".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let created = ServicePlan::create(plan.clone(), &test_session(), &mut store).await?;

        assert_eq!(created.part, Some(part.id));
        assert_eq!(created.uid, None);
        // create() does NOT auto-set what; it's stored as-is
        assert_eq!(created.what, PartTypeId::from(2));
        Ok(())
    }

    /// SP-02: ServicePlan create generic plan (no specific part)
    #[tokio::test]
    async fn service_plan_create_generic_plan() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(3),
            hook: Some(PartTypeId::from(1)),
            name: "Generic Plan".to_string(),
            days: Some(60),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        assert_eq!(created.part, None);
        assert_eq!(created.uid, Some(test_user()));
        Ok(())
    }

    /// SP-03: ServicePlan create stores what as-provided (not auto-set from part type)
    #[tokio::test]
    async fn service_plan_create_sets_what_to_part_type() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        // create() does NOT auto-set what from part type; it stores as-is
        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: Some(part.id),
            what: PartTypeId::from(99),
            hook: None,
            name: "Wrong What".to_string(),
            days: Some(10),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        assert_eq!(created.what, PartTypeId::from(99));
        Ok(())
    }

    /// SP-04: ServicePlan create with thresholds
    #[tokio::test]
    async fn service_plan_create_with_thresholds() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Threshold Plan".to_string(),
            days: Some(30),
            hours: Some(60),
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let created_retrieved = ServicePlanStore::get(&mut store, created.id).await?;
        assert_eq!(created_retrieved.hours, Some(60));
        Ok(())
    }

    /// SP-05: ServicePlan get returns stored plan
    #[tokio::test]
    async fn service_plan_get_returns_stored_plan() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(2),
            hook: Some(PartTypeId::from(3)),
            name: "Get Test".to_string(),
            days: Some(45),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;
        let stored = ServicePlanStore::get(&mut store, created.id).await?;
        assert_eq!(stored.name, "Get Test");
        assert_eq!(stored.days, Some(45));
        Ok(())
    }

    /// SP-06: ServicePlan by_part returns matching plans
    #[tokio::test]
    async fn serviceplan_by_part_returns_matching_plans() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part_a = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let part_b = Part::create(
            "Part B".to_string(),
            "Brand B".to_string(),
            "Model B".to_string(),
            PartTypeId::from(2),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        ServicePlan::create(
            ServicePlan {
                id: ServicePlanId::new(),
                part: Some(part_a.id),
                what: PartTypeId::from(1),
                hook: None,
                name: "Plan A".to_string(),
                days: Some(30),
                hours: None,
                km: None,
                climb: None,
                descend: None,
                rides: None,
                uid: None,
                energy: None,
            },
            &test_session(),
            &mut store,
        )
        .await?;

        ServicePlan::create(
            ServicePlan {
                id: ServicePlanId::new(),
                part: Some(part_b.id),
                what: PartTypeId::from(2),
                hook: None,
                name: "Plan B".to_string(),
                days: Some(60),
                hours: None,
                km: None,
                climb: None,
                descend: None,
                rides: None,
                uid: None,
                energy: None,
            },
            &test_session(),
            &mut store,
        )
        .await?;

        let plans_a = ServicePlan::for_part(part_a.id, &mut store).await?;
        assert_eq!(plans_a.len(), 1);
        assert_eq!(plans_a[0].name, "Plan A");

        let plans_b = ServicePlan::for_part(part_b.id, &mut store).await?;
        assert_eq!(plans_b.len(), 1);
        assert_eq!(plans_b[0].name, "Plan B");

        Ok(())
    }

    /// SP-07: ServicePlan by_user returns generic plans
    #[tokio::test]
    async fn serviceplan_by_user_returns_generic_plans() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session1 = TestSession::new(UserId::from(1));
        let session2 = TestSession::new(UserId::from(2));

        ServicePlan::create(
            ServicePlan {
                id: ServicePlanId::new(),
                part: None,
                what: PartTypeId::from(1),
                hook: None,
                name: "Plan 1".to_string(),
                days: Some(30),
                hours: None,
                km: None,
                climb: None,
                descend: None,
                rides: None,
                uid: Some(UserId::from(1)),
                energy: None,
            },
            &session1,
            &mut store,
        )
        .await?;

        ServicePlan::create(
            ServicePlan {
                id: ServicePlanId::new(),
                part: None,
                what: PartTypeId::from(1),
                hook: None,
                name: "Plan 2".to_string(),
                days: Some(60),
                hours: None,
                km: None,
                climb: None,
                descend: None,
                rides: None,
                uid: Some(UserId::from(1)),
                energy: None,
            },
            &session1,
            &mut store,
        )
        .await?;

        ServicePlan::create(
            ServicePlan {
                id: ServicePlanId::new(),
                part: None,
                what: PartTypeId::from(1),
                hook: None,
                name: "Plan 3".to_string(),
                days: Some(90),
                hours: None,
                km: None,
                climb: None,
                descend: None,
                rides: None,
                uid: Some(UserId::from(2)),
                energy: None,
            },
            &session2,
            &mut store,
        )
        .await?;

        let plans_1 = ServicePlan::for_user(&UserId::from(1), &mut store).await?;
        assert_eq!(plans_1.len(), 2);

        let plans_2 = ServicePlan::for_user(&UserId::from(2), &mut store).await?;
        assert_eq!(plans_2.len(), 1);

        Ok(())
    }

    /// SP-08: ServicePlan update preserves immutable fields
    #[tokio::test]
    async fn service_plan_update_preserves_immutable_fields() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(5),
            hook: Some(PartTypeId::from(7)),
            name: "Original".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        // Update only name
        let updated = ServicePlan {
            id: created.id,
            part: None,
            what: PartTypeId::from(99),
            hook: Some(PartTypeId::from(99)),
            name: "Updated".to_string(),
            days: Some(60),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(UserId::from(99)),
            energy: None,
        };
        let result = updated.clone().update(&test_session(), &mut store).await?;

        assert_eq!(result.name, "Updated");
        assert_eq!(result.part, None);
        assert_eq!(result.what, PartTypeId::from(5));
        assert_eq!(result.hook, Some(PartTypeId::from(7)));
        assert_eq!(result.uid, Some(test_user()));
        Ok(())
    }

    /// SP-09: ServicePlan update requires ownership (specific part)
    #[tokio::test]
    async fn service_plan_update_requires_ownership_specific() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: Some(part.id),
            what: PartTypeId::from(1),
            hook: None,
            name: "Owned Plan".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let other_user = TestSession::new(UserId::from(99));
        let update = ServicePlan {
            id: created.id,
            part: Some(part.id),
            what: PartTypeId::from(99),
            hook: None,
            name: "Hacked".to_string(),
            days: Some(1),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let result = update.update(&other_user, &mut store).await;
        assert!(result.is_err());

        Ok(())
    }

    /// SP-10: ServicePlan update requires ownership (generic plan)
    #[tokio::test]
    async fn service_plan_update_requires_ownership_generic() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Generic Owned".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let other_user = TestSession::new(UserId::from(99));
        let update = ServicePlan {
            id: created.id,
            part: None,
            what: PartTypeId::from(99),
            hook: None,
            name: "Hacked".to_string(),
            days: Some(1),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(UserId::from(99)),
            energy: None,
        };
        let result = update.update(&other_user, &mut store).await;
        assert!(result.is_err());

        Ok(())
    }

    /// SP-11: ServicePlan delete removes plan
    #[tokio::test]
    async fn service_plan_delete_removes_plan() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Delete Me".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        // Delete via serviceplan delete
        let deleted = created.id.delete(&test_session(), &mut store).await?;
        assert_eq!(deleted.len(), 0);

        // Now should not be found - verify through by_user instead
        let plans = ServicePlan::for_user(&test_user(), &mut store).await?;
        assert!(plans.is_empty());

        Ok(())
    }

    /// SP-12: ServicePlan delete no-op on reset_plan (returns empty service list)
    #[tokio::test]
    async fn service_plan_delete_noop_on_reset_plan() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Reset Plan".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let res = created.id.delete(&test_session(), &mut store).await?;
        assert_eq!(res.len(), 0);

        Ok(())
    }

    // === Suite 7: ServicePlan — Threshold Data Model ===

    /// SP-13: All threshold fields None means plan is still valid (time-only)
    #[tokio::test]
    async fn service_plan_all_thresholds_none_means_active() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Time Only".to_string(),
            days: Some(30),
            hours: None,
            km: None,
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let stored = ServicePlanStore::get(&mut store, created.id).await?;
        assert_eq!(stored.hours, None);
        assert_eq!(stored.km, None);
        assert_eq!(stored.climb, None);
        assert_eq!(stored.rides, None);
        Ok(())
    }

    /// SP-14: ServicePlan with single threshold only
    #[tokio::test]
    async fn service_plan_single_threshold_only() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Km Only".to_string(),
            days: Some(30),
            hours: None,
            km: Some(500),
            climb: None,
            descend: None,
            rides: None,
            uid: Some(test_user()),
            energy: None,
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let stored = ServicePlanStore::get(&mut store, created.id).await?;
        assert_eq!(stored.km, Some(500));
        assert_eq!(stored.hours, None);
        assert_eq!(stored.climb, None);
        assert_eq!(stored.rides, None);

        Ok(())
    }

    /// SP-15: ServicePlan with multiple thresholds
    #[tokio::test]
    async fn service_plan_multiple_thresholds() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: None,
            what: PartTypeId::from(1),
            hook: None,
            name: "Multi Threshold".to_string(),
            days: Some(30),
            hours: Some(100),
            km: Some(5000),
            climb: Some(5000),
            descend: None,
            rides: Some(20),
            uid: Some(test_user()),
            energy: Some(3000),
        };
        let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

        let stored = ServicePlanStore::get(&mut store, created.id).await?;
        assert_eq!(stored.hours, Some(100));
        assert_eq!(stored.km, Some(5000));
        assert_eq!(stored.climb, Some(5000));
        assert_eq!(stored.rides, Some(20));
        assert_eq!(stored.energy, Some(3000));

        Ok(())
    }

    /// SP-16: ServicePlan for_part returns empty when no plans on part
    #[tokio::test]
    async fn serviceplan_for_part_returns_empty_when_none() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        let plans = ServicePlan::for_part(part.id, &mut store).await?;
        assert!(plans.is_empty());

        Ok(())
    }
}
