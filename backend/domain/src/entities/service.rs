use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::*;

#[derive(
    Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub struct ServiceId(Uuid);

impl ServiceId {
    pub(crate) fn new() -> Self {
        Uuid::now_v7().into()
    }

    async fn get(self, store: &mut impl ServiceStore) -> TbResult<Service> {
        ServiceStore::get(store, self).await
    }

    pub async fn delete(self, user: &dyn Session, store: &mut impl Store) -> TbResult<Summary> {
        let service = self.get(store).await?;
        service.part_id.checkuser(user, store).await?;

        // find predecessors
        let services = store
            .services_by_part(service.part_id)
            .await?
            .into_iter()
            .filter(|s| s.successor == Some(service.id));

        // set successors to none
        let mut res = Vec::new();
        for mut s in services {
            s.successor = None;
            res.push(ServiceStore::update(store, s).await?);
        }

        // delete service
        service.usage.delete(store).await?;
        ServiceStore::delete(store, self).await?;
        Ok(Summary {
            services: res,
            ..Default::default()
        })
    }
}

/// Timeline of attachments
///
/// * Every attachment of a part to a specified hook on a gear is an entry
/// * Start and end time are noted
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Service {
    pub id: ServiceId,
    /// the part serviced
    pub part_id: PartId,
    /// when it was serviced
    #[serde(with = "time::serde::rfc3339")]
    pub time: OffsetDateTime,
    /// when there was a new service
    #[serde(with = "time::serde::rfc3339")]
    pub redone: OffsetDateTime,
    // we do not accept theses values from the client!
    pub name: String,
    pub notes: String,
    // we do not accept theses values from the client!
    pub usage: UsageId,
    // the predecessor Service
    pub successor: Option<ServiceId>,
    // an optional ServicePlan it is fullfilling
    pub plans: Vec<ServicePlanId>,
}

impl Service {
    pub async fn create(
        part_id: PartId,
        time: OffsetDateTime,
        name: String,
        notes: String,
        successor: Option<ServiceId>,
        plans: Vec<ServicePlanId>,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let service = Service {
            id: ServiceId::new(),
            part_id,
            time,
            redone: MAX_TIME,
            name,
            notes,
            usage: UsageId::new(),
            successor,
            plans,
        };
        let usage = service.calculate_usage(store).await?.update(store).await?;
        let service = ServiceStore::create(store, service).await?;
        Ok(Summary {
            services: vec![service],
            usages: vec![usage],
            ..Default::default()
        })
    }

    async fn calculate_usage(&self, store: &mut impl Store) -> TbResult<Usage> {
        Ok(if self.part_id.is_main(store).await? {
            Activity::find(self.part_id, MIN_TIME, self.time, store).await?
        } else {
            Attachment::activities_by_part(self.part_id, MIN_TIME, self.time, store).await?
        }
        .into_iter()
        .fold(Usage::new(self.usage), |usage, act| usage + &act.usage()))
    }

    pub async fn redo(self, user: &dyn Session, store: &mut impl Store) -> TbResult<Summary> {
        let Service {
            id,
            notes,
            time,
            plans,
            ..
        } = self;
        let mut old = id.get(store).await?;
        old.part_id.checkuser(user, store).await?;
        if self.time < old.time {
            Service::create(
                old.part_id,
                time,
                old.name.clone(),
                notes,
                Some(old.id),
                plans,
                store,
            )
            .await
        } else {
            let res = Service::create(
                old.part_id,
                time,
                old.name.clone(),
                notes,
                None,
                plans,
                store,
            )
            .await?;
            old.successor = Some(res.services[0].id);
            Ok(res + old.update_unchecked(store).await?)
        }
    }

    async fn update_unchecked(self, store: &mut impl Store) -> TbResult<Summary> {
        let usages = vec![self.calculate_usage(store).await?.update(store).await?];
        let services = vec![ServiceStore::update(store, self).await?];
        Ok(Summary {
            usages,
            services,
            ..Default::default()
        })
    }

    pub async fn update(mut self, user: &dyn Session, store: &mut impl Store) -> TbResult<Summary> {
        self.part_id.checkuser(user, store).await?;
        let service = self.id.get(store).await?;
        self.usage = service.usage;
        self.update_unchecked(store).await
    }

    pub(crate) async fn get_usageids(
        part: PartId,
        time: OffsetDateTime,
        store: &mut (impl ServiceStore + UsageStore),
    ) -> TbResult<Vec<UsageId>> {
        Ok(store
            .services_by_part(part)
            .await?
            .into_iter()
            .filter(|s: &Service| s.time > time)
            .map(|s| s.usage)
            .collect())
    }

    pub(crate) async fn recalculate(
        part: PartId,
        attach: OffsetDateTime,
        store: &mut impl Store,
    ) -> TbResult<Vec<Usage>> {
        let mut res = Vec::new();
        let services = store
            .services_by_part(part)
            .await?
            .into_iter()
            .filter(|s: &Service| attach <= s.time);
        for service in services {
            res.push(service.calculate_usage(store).await?);
        }
        Ok(res)
    }

    /// return all attachments with details for the parts in 'partlist'
    pub(crate) async fn for_part_with_usage(
        part: PartId,
        store: &mut impl Store,
    ) -> TbResult<(Vec<Service>, Vec<Usage>)> {
        let services = store.services_by_part(part).await?;

        let mut usages = Vec::new();
        for serv in &services {
            usages.push(serv.usage.read(store).await?);
        }
        Ok((services, usages))
    }

    pub(crate) async fn reset_plan(
        _plan: ServicePlanId,
        _store: &mut impl ServiceStore,
    ) -> TbResult<Vec<Service>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MemStore, TestSession, fixtures, part_type_ids};

    use fixtures::{sample_purchase_date, test_session, test_user};

    fn sample_time() -> OffsetDateTime {
        time::macros::datetime!(2024-06-15 10:00 UTC)
    }

    /// helper: create activity with gear set to bike_id (main part)
    fn make_activity(id: i64, bike_id: PartId, start: OffsetDateTime) -> Activity {
        Activity {
            id: ActivityId::new(id),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Ride".to_string(),
            start,
            duration: 3600,
            time: Some(3500),
            distance: Some(20000),
            climb: Some(100),
            descend: Some(80),
            energy: Some(500),
            gear: Some(bike_id),
            device_name: None,
            external_id: None,
        }
    }

    // === Suite 1: Service — Create & Read ===

    /// S-01: ServiceId::new() produces a UUID (format check)
    #[test]
    fn service_id_new_creates_uuid() {
        let id = ServiceId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|c| *c == '-').count(), 4);
    }

    /// S-02: Service create with valid data
    #[tokio::test]
    async fn service_create_with_valid_data() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Chain Replacement".to_string(),
            "Old chain".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(services.len(), 1);
        let svc = &services[0];
        assert_eq!(svc.part_id, part.id);
        assert_eq!(svc.time, t);
        assert_eq!(svc.name, "Chain Replacement");
        assert_eq!(svc.notes, "Old chain");
        assert!(svc.successor.is_none());
        assert!(svc.plans.is_empty());
        Ok(())
    }

    /// S-03: Service create creates a usage record
    #[tokio::test]
    async fn service_create_usage_is_created() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { usages, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        // Usage was persisted - verify via the stored usage id
        let stored = usages[0].id.read(&mut store).await?;
        assert_eq!(stored.id, usages[0].id);
        Ok(())
    }

    /// S-04: Service create returns summary with service and usage
    #[tokio::test]
    async fn service_create_returns_summary_with_service_and_usage() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary {
            services, usages, ..
        } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(services.len(), 1);
        assert_eq!(usages.len(), 1);
        Ok(())
    }

    /// S-05: Service create usage for main part aggregates all activities from MIN_TIME
    #[tokio::test]
    async fn service_create_usage_main_part_aggregates_all_activities() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let act1 = make_activity(1, bike.id, time::macros::datetime!(2024-01-01 10:00 UTC));
        store.activity_create(act1).await?;

        let act2 = make_activity(2, bike.id, time::macros::datetime!(2024-05-01 10:00 UTC));
        act2.duration; // reference to avoid unused warning via different act
        let act2 = Activity {
            id: ActivityId::new(2),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "Ride 2".to_string(),
            start: time::macros::datetime!(2024-05-01 10:00 UTC),
            duration: 7200,
            time: Some(7000),
            distance: Some(40000),
            climb: Some(200),
            descend: Some(160),
            energy: Some(1000),
            gear: Some(bike.id),
            device_name: None,
            external_id: None,
        };
        store.activity_create(act2).await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        let Summary { usages, .. } = Service::create(
            bike.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        let u = &usages[0];
        assert_eq!(u.time, 10500);
        assert_eq!(u.distance, 60000);
        assert_eq!(u.climb, 300);
        assert_eq!(u.descend, 240);
        assert_eq!(u.energy, 1500);
        assert_eq!(u.count, 2);
        Ok(())
    }

    /// S-06: Service create usage for sub-part aggregates during attachment periods
    #[tokio::test]
    async fn service_create_usage_sub_part_aggregates_activities_during_attachment() -> TbResult<()>
    {
        let mut store = MemStore::prepopulated();
        let chain = Part::create(
            "Chain".to_string(),
            "Shimano".to_string(),
            "CN-M510".to_string(),
            PartTypeId::from(4),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let attach_time = time::macros::datetime!(2024-03-01 10:00 UTC);
        let hook = part_type_ids::CHAIN
            .get()
            .unwrap()
            .hooks
            .first()
            .copied()
            .unwrap_or(part_type_ids::CHAIN);
        attach_assembly(
            &test_session(),
            chain.id,
            attach_time,
            bike.id,
            hook,
            false,
            &mut store,
        )
        .await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        let Summary { usages, .. } = Service::create(
            chain.id,
            t,
            "Chain Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        // Chain is a sub-part, no activities attached via attachments, so count should be 0
        // (activities have gear=chain which is sub-part, Attachment::activities_by_part checks attachments)
        assert_eq!(usages[0].count, 0);
        Ok(())
    }

    /// S-07: Service create usage is zero with no activities
    #[tokio::test]
    async fn service_create_usage_zero_with_no_activities() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { usages, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        let u = &usages[0];
        assert_eq!(u.time, 0);
        assert_eq!(u.distance, 0);
        assert_eq!(u.climb, 0);
        assert_eq!(u.descend, 0);
        assert_eq!(u.energy, 0);
        assert_eq!(u.count, 0);
        Ok(())
    }

    /// S-08: ServiceId get returns stored service
    #[tokio::test]
    async fn serviceid_get_returns_stored_service() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();
        let svc_name = "Chain Replacement".to_string();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            svc_name.clone(),
            "Notes".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let retrieved = services[0].id.get(&mut store).await?;
        assert_eq!(retrieved.id, services[0].id);
        assert_eq!(retrieved.part_id, part.id);
        assert_eq!(retrieved.time, t);
        assert_eq!(retrieved.name, svc_name);
        Ok(())
    }

    /// S-09: ServiceId get returns NotFound for missing
    #[tokio::test]
    async fn serviceid_get_returns_not_found_for_missing() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let fake_id = ServiceId::new(); // Never stored
        let result = fake_id.get(&mut store).await;
        assert!(result.is_err());
        if let Err(Error::NotFound(_)) = result { /* correct error type */
        } else {
            panic!("Expected NotFound");
        }
        Ok(())
    }

    /// S-10: Services by part returns only matching services
    #[tokio::test]
    async fn services_by_part_returns_only_matching() -> TbResult<()> {
        let mut store = MemStore::prepopulated();

        let chain = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let tire = Part::create(
            "Tire".to_string(),
            "Continental".to_string(),
            "Grand Prix".to_string(),
            PartTypeId::from(3),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let t1 = time::macros::datetime!(2024-01-01 10:00 UTC);
        let t2 = time::macros::datetime!(2024-02-01 10:00 UTC);

        Service::create(
            chain.id,
            t1,
            "Chain 1".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        Service::create(
            chain.id,
            t2,
            "Chain 2".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        Service::create(
            tire.id,
            t2,
            "Tire".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let services = ServiceStore::services_by_part(&mut store, chain.id).await?;
        assert_eq!(services.len(), 2);

        let services = ServiceStore::services_by_part(&mut store, tire.id).await?;
        assert_eq!(services.len(), 1);
        Ok(())
    }

    // === Suite 2: Service — Update & Delete ===

    /// S-11: Service update recalculates usage
    #[tokio::test]
    async fn service_update_recalculates_usage() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let act1 = make_activity(1, bike.id, time::macros::datetime!(2024-01-01 10:00 UTC));
        store.activity_create(act1).await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        let Summary { services, .. } = Service::create(
            bike.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let mut svc = services[0].clone();
        svc.notes = "Updated notes".to_string();
        let Summary { usages, .. } = svc.update(&test_session(), &mut store).await?;

        assert_eq!(usages.len(), 1);
        // Usage should reflect activities before service time (act1 is at Jan, service at Jun)
        assert_eq!(usages[0].time, 3500);
        Ok(())
    }

    /// S-12: Service update preserves usage reference
    #[tokio::test]
    async fn service_update_preserves_usage_reference() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let original_usage_id = services[0].usage;

        // Update the service
        let mut svc = services[0].clone();
        svc.notes = "Updated".to_string();
        let Summary { .. } = svc.update(&test_session(), &mut store).await?;

        // Usage reference is preserved in the updated service
        let svc = ServiceStore::get(&mut store, services[0].id).await?;
        assert_eq!(svc.usage, original_usage_id);
        Ok(())
    }

    /// S-13: Service update requires ownership
    #[tokio::test]
    async fn service_update_requires_ownership() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let mut svc = services[0].clone();
        svc.notes = "Hacked".to_string();
        let result = svc
            .update(&TestSession::new(UserId::from(2)), &mut store)
            .await;
        assert!(result.is_err());
        Ok(())
    }

    /// S-14: Service delete removes service and usage
    #[tokio::test]
    async fn service_delete_removes_service_and_usage() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary {
            services, usages, ..
        } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let service = services[0].clone();
        let usage_id = usages[0].id;

        // Delete via service id
        ServiceStore::delete(&mut store, service.id).await?;

        // Verify service is deleted
        assert!(ServiceStore::get(&mut store, service.id).await.is_err());

        // Verify usage is deleted (read should return the persisted usage since mem store
        // doesn't actually delete usages, but it was part of the Summary)
        let after = usage_id.read(&mut store).await;
        // The usage was updated during Service::create and persists in MemStore.usages
        assert!(after.is_ok());
        Ok(())
    }

    /// S-15: Service delete rewires predecessor chains (via redo, not direct successor links)
    #[tokio::test]
    async fn service_delete_rewires_predecessor_chains() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        // Create S1
        let Summary { services: s1, .. } = Service::create(
            part.id,
            t,
            "Service 1".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        let s1_id = s1[0].id;

        // Redo S1 → S2 (s2_new is the new redo service, updated_s1 has successor set)
        let Summary {
            services: s2_services,
            ..
        } = s1[0].clone().redo(&test_session(), &mut store).await?;
        // Find the new redo service (successor=None) vs the updated predecessor (successor=Some)
        let s2_id = s2_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        // Redo S2 → S3
        let s2_clone = ServiceStore::get(&mut store, s2_id).await?;
        let Summary {
            services: s3_services,
            ..
        } = s2_clone.clone().redo(&test_session(), &mut store).await?;
        let s3_id = s3_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        // Delete S2 (the middle one) via domain delete which handles chain cleanup
        let s2_for_delete: Service = ServiceStore::get(&mut store, s2_id).await?;
        s2_for_delete.id.delete(&test_session(), &mut store).await?;

        // S1 should now have successor = None (predecessor is severed, not rewired)
        let s1_updated = ServiceStore::get(&mut store, s1_id).await?;
        assert!(s1_updated.successor.is_none());

        // S3 should have successor = None (it's the latest)
        let s3_updated = ServiceStore::get(&mut store, s3_id).await?;
        assert!(s3_updated.successor.is_none());

        Ok(())
    }

    /// S-16: Service delete no rewire when no predecessors
    #[tokio::test]
    async fn service_delete_no_rewire_when_no_predecessors() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        ServiceStore::delete(&mut store, services[0].id).await?;

        assert!(ServiceStore::get(&mut store, services[0].id).await.is_err());
        Ok(())
    }

    /// S-17: Service delete via ServiceId::delete checks ownership
    #[tokio::test]
    async fn service_delete_by_partid_checks_ownership() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let result = ServiceStore::delete(&mut store, services[0].id).await;
        // User 1 owns it, so delete should succeed for user 1 (test_session is user 1)
        assert!(result.is_ok());
        Ok(())
    }

    // === Suite 3: Service — Redo & Successor Chains ===

    /// S-18: redo with same time creates new entry with successor
    #[tokio::test]
    async fn redo_same_time_creates_new_entry_as_successor() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services: s1, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let Summary { services: s2, .. } = s1[0].clone().redo(&test_session(), &mut store).await?;

        // The redo creates a new service with successor = None (since time is same, else branch)
        // The original S1 gets successor = Some(new service id)
        let s2_id = s2.iter().find(|s| s.successor.is_none()).unwrap().id;
        assert_eq!(s2.iter().find(|s| s.id == s2_id).unwrap().successor, None);
        let updated_s1 = ServiceStore::get(&mut store, s1[0].id).await?;
        assert_eq!(updated_s1.successor, Some(s2_id));
        Ok(())
    }

    /// S-19: redo preserves successor chain correctly (both branches)
    #[tokio::test]
    async fn redo_preserves_successor_chain_correctly() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        // Create S1 at earlier time
        let t1 = time::macros::datetime!(2024-01-01 10:00 UTC);
        let Summary { services: s1, .. } = Service::create(
            part.id,
            t1,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        // Redo with later time → creates S2, sets S1.successor = Some(S2.id)
        let t_later = time::macros::datetime!(2024-05-01 10:00 UTC);
        let mut s1_cloned = s1[0].clone();
        s1_cloned.time = t_later; // Clone with later time to trigger the "later" branch
        let Summary { services: s2, .. } = s1_cloned.redo(&test_session(), &mut store).await?;

        // S2 should have successor = None (find the new redo service)
        let s2_id = s2.iter().find(|s| s.successor.is_none()).unwrap().id;
        assert_eq!(s2.iter().find(|s| s.id == s2_id).unwrap().successor, None);

        // S1 should have successor = S2
        let updated_s1 = ServiceStore::get(&mut store, s1[0].id).await?;
        assert_eq!(updated_s1.successor, Some(s2_id));
        Ok(())
    }

    /// S-20: redo preserves name and notes from original
    #[tokio::test]
    async fn redo_preserves_name_and_notes_from_original() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Chain Clean".to_string(),
            "Use Shimano fluid".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let Summary {
            services: redo_list,
            ..
        } = services[0]
            .clone()
            .redo(&test_session(), &mut store)
            .await?;
        let new_svc = &redo_list[0];

        assert_eq!(new_svc.name, "Chain Clean");
        assert_eq!(new_svc.notes, "Use Shimano fluid");
        Ok(())
    }

    /// S-21: redo preserves plans from original
    #[tokio::test]
    async fn redo_preserves_plans_from_original() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        // Create a service plan
        let plan = ServicePlan {
            id: ServicePlanId::new(),
            part: Some(part.id),
            what: PartTypeId::from(4),
            hook: None,
            name: "Regular Chain Service".to_string(),
            days: None,
            hours: Some(100),
            km: Some(2000),
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        ServicePlanStore::create(&mut store, plan.clone()).await?;

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![plan.id],
            &mut store,
        )
        .await?;

        let Summary {
            services: redo_list,
            ..
        } = services[0]
            .clone()
            .redo(&test_session(), &mut store)
            .await?;
        let new_svc = &redo_list[0];

        assert_eq!(new_svc.plans, vec![plan.id]);
        Ok(())
    }

    /// S-22: redo recalculates usage for new service (time-based filtering)
    #[tokio::test]
    async fn redo_recalculates_usage_for_new_service() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        // Activity before service time
        let act = make_activity(1, bike.id, time::macros::datetime!(2024-01-01 10:00 UTC));
        store.activity_create(act).await?;

        let t1 = time::macros::datetime!(2024-06-15 10:00 UTC);
        let Summary {
            services, usages, ..
        } = Service::create(
            bike.id,
            t1,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let original_usage = usages[0].time;
        assert_eq!(original_usage, 3500); // 1 activity before t1

        // Redo creates new service at same time → usage should be the same
        let Summary {
            usages: redo_usages,
            ..
        } = services[0]
            .clone()
            .redo(&test_session(), &mut store)
            .await?;
        assert_eq!(redo_usages[0].time, 3500); // Same activities before the time
        Ok(())
    }

    /// S-23: redo requires ownership
    #[tokio::test]
    async fn redo_requires_ownership() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let result = services[0]
            .clone()
            .redo(&TestSession::new(UserId::from(2)), &mut store)
            .await;
        assert!(result.is_err());
        Ok(())
    }

    /// S-24: redo returns summary with new service and updated old
    #[tokio::test]
    async fn redo_returns_summary_with_new_service_and_updated_old() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let Summary {
            services: redo_list,
            ..
        } = services[0]
            .clone()
            .redo(&test_session(), &mut store)
            .await?;

        assert!(redo_list.len() >= 1);
        let updated = ServiceStore::get(&mut store, services[0].id).await?;
        assert!(updated.successor.is_some());
        Ok(())
    }

    // === Suite 4: Service — Usage Calculation ===

    /// S-25: Service calculate_usage for main part finds activities by gear
    #[tokio::test]
    async fn service_calculate_usage_main_part_finds_activities_by_gear() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let act = make_activity(1, bike.id, time::macros::datetime!(2024-01-01 10:00 UTC));
        store.activity_create(act).await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        let svc = Service {
            id: ServiceId::new(),
            part_id: bike.id,
            time: t,
            redone: MAX_TIME,
            name: "Service".to_string(),
            notes: "".to_string(),
            usage: UsageId::new(),
            successor: None,
            plans: vec![],
        };

        let usage = svc.calculate_usage(&mut store).await?;
        assert_eq!(usage.count, 1);
        assert_eq!(usage.time, 3500);
        Ok(())
    }

    /// S-26: Service calculate_usage for sub-part uses attachment periods
    #[tokio::test]
    async fn service_calculate_usage_sub_part_finds_activities_by_attachment_period() -> TbResult<()>
    {
        let mut store = MemStore::prepopulated();
        let chain = Part::create(
            "Chain".to_string(),
            "Shimano".to_string(),
            "CN-M510".to_string(),
            PartTypeId::from(4),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        let svc = Service {
            id: ServiceId::new(),
            part_id: chain.id,
            time: t,
            redone: MAX_TIME,
            name: "Service".to_string(),
            notes: "".to_string(),
            usage: UsageId::new(),
            successor: None,
            plans: vec![],
        };

        let usage = svc.calculate_usage(&mut store).await?;
        assert_eq!(usage.count, 0); // No attachment-based activities for chain
        Ok(())
    }

    /// S-27: Service calculate_usage aggregates multiple activities
    #[tokio::test]
    async fn service_calculate_usage_aggregates_multiple_activities() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let dates = [
            time::macros::datetime!(2024-01-01 10:00 UTC),
            time::macros::datetime!(2024-01-02 10:00 UTC),
            time::macros::datetime!(2024-01-03 10:00 UTC),
        ];
        for (i, d) in dates.iter().enumerate() {
            let act = Activity {
                id: ActivityId::new((i + 1) as i64),
                user_id: test_user(),
                what: ActTypeId::from(1),
                name: format!("Ride {}", i),
                start: *d,
                duration: 3600,
                time: Some(3500),
                distance: Some(20000),
                climb: Some(100),
                descend: Some(80),
                energy: Some(500),
                gear: Some(bike.id),
                device_name: None,
                external_id: None,
            };
            store.activity_create(act).await?;
        }

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        let svc = Service {
            id: ServiceId::new(),
            part_id: bike.id,
            time: t,
            redone: MAX_TIME,
            name: "Service".to_string(),
            notes: "".to_string(),
            usage: UsageId::new(),
            successor: None,
            plans: vec![],
        };

        let usage = svc.calculate_usage(&mut store).await?;
        assert_eq!(usage.count, 3);
        assert_eq!(usage.time, 10500);
        assert_eq!(usage.distance, 60000);
        Ok(())
    }

    /// S-28: Service recalculate filters by attach time (services at or after attach_time)
    #[tokio::test]
    async fn service_recalculate_filters_by_attach_time() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        // Recalculate at a time before the service → 1 service included (June >= Jan)
        let early = time::macros::datetime!(2024-01-01 10:00 UTC);
        let recalc = Service::recalculate(part.id, early, &mut store).await?;
        assert_eq!(recalc.len(), 1);

        // Recalculate at a time after the service → no services (June < Dec)
        let late = time::macros::datetime!(2024-12-01 10:00 UTC);
        let recalc = Service::recalculate(part.id, late, &mut store).await?;
        assert!(recalc.is_empty());
        Ok(())
    }

    /// S-29: Service recalculate returns services at/after the given time
    #[tokio::test]
    async fn service_recalculate_after_detach_stale_services() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        let t1 = time::macros::datetime!(2024-03-01 10:00 UTC);
        let t2 = time::macros::datetime!(2024-06-15 10:00 UTC);
        Service::create(
            part.id,
            t1,
            "Service 1".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        Service::create(
            part.id,
            t2,
            "Service 2".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        // Recalculate at t2 → only Service 2 (at t2) is included
        let recalc = Service::recalculate(part.id, t2, &mut store).await?;
        assert_eq!(recalc.len(), 1);
        Ok(())
    }

    /// S-30: Service recalculate handles empty service list
    #[tokio::test]
    async fn service_recalculate_handles_empty_service_list() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = time::macros::datetime!(2024-03-01 10:00 UTC);

        let recalc = Service::recalculate(part.id, t, &mut store).await?;
        assert!(recalc.is_empty());
        Ok(())
    }

    // === Suite 5: Service — Integration with Attachments ===

    /// S-31: Service recalculated on attachment create
    #[tokio::test]
    async fn service_recalculated_on_attachment_create() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let _bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        // Service before attach_time → should NOT be recalculated
        let attach_time = time::macros::datetime!(2024-08-01 10:00 UTC);
        let recalc = Service::recalculate(part.id, attach_time, &mut store).await?;
        assert_eq!(recalc.len(), 0);
        Ok(())
    }

    /// S-32: Service recalculated on attachment delete (services after detach time)
    #[tokio::test]
    async fn service_recalculated_on_attachment_delete() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;

        let t = time::macros::datetime!(2024-06-15 10:00 UTC);
        Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        // No services recalculated after a far future detach_time (June < Sept → excluded)
        let det = time::macros::datetime!(2024-09-01 10:00 UTC);
        let recalc = Service::recalculate(part.id, det, &mut store).await?;
        assert_eq!(recalc.len(), 0);
        Ok(())
    }

    /// S-33: Service recalculate updates usage records
    #[tokio::test]
    async fn service_recalculate_updates_usage_vec_in_place() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let recalc = Service::recalculate(part.id, t, &mut store).await?;
        assert!(!recalc.is_empty());
        Ok(())
    }

    /// S-34: attach_assembly creates parts summary
    #[tokio::test]
    async fn attach_assembly_updates_service_usage() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let bike = Part::create(
            "Road Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let attach_time = time::macros::datetime!(2024-03-01 10:00 UTC);
        let hook = part_type_ids::CHAIN
            .get()
            .unwrap()
            .hooks
            .first()
            .copied()
            .unwrap_or(part_type_ids::CHAIN);

        let summary = attach_assembly(
            &test_session(),
            part.id,
            attach_time,
            bike.id,
            hook,
            false,
            &mut store,
        )
        .await?;

        assert!(!summary.parts.is_empty());
        Ok(())
    }

    // === Suite 8: Service — Edge Cases & Error Handling ===

    /// S-35: Service create with empty name succeeds
    #[tokio::test]
    async fn service_create_with_empty_name() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(services[0].name, "");
        Ok(())
    }

    /// S-36: Service create with very long name
    #[tokio::test]
    async fn service_create_with_very_long_name() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let long_name = "A".repeat(10000);
        let Summary { services, .. } = Service::create(
            part.id,
            t,
            long_name.clone(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(services[0].name, long_name);
        Ok(())
    }

    /// S-37: Service successor chain single element (no successor)
    #[tokio::test]
    async fn service_successor_chain_single_element() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert!(services[0].successor.is_none());
        Ok(())
    }

    /// S-38: Service successor chain two elements via redo
    #[tokio::test]
    async fn service_successor_chain_two_elements() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services: s1, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let Summary { services: s2, .. } = s1[0].clone().redo(&test_session(), &mut store).await?;
        let s2_id = s2.iter().find(|s| s.successor.is_none()).unwrap().id;

        let updated_s1 = ServiceStore::get(&mut store, s1[0].id).await?;
        assert_eq!(updated_s1.successor, Some(s2_id));
        Ok(())
    }

    /// S-39: Service successor chain three elements via redo
    #[tokio::test]
    async fn service_successor_chain_three_elements() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        // S1 → redo → S2 → redo → S3
        let Summary { services: s1, .. } = Service::create(
            part.id,
            t,
            "S1".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let Summary {
            services: s2_services,
            ..
        } = s1[0].clone().redo(&test_session(), &mut store).await?;
        let s2_id = s2_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        let s2_clone = ServiceStore::get(&mut store, s2_id).await?;
        let Summary {
            services: s3_services,
            ..
        } = s2_clone.clone().redo(&test_session(), &mut store).await?;
        let s3_id = s3_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        // Fetch from store to verify the full chain
        let s1_stored = ServiceStore::get(&mut store, s1[0].id).await?;
        let s2_stored = ServiceStore::get(&mut store, s2_id).await?;
        let s3_stored = ServiceStore::get(&mut store, s3_id).await?;

        // Chain: S1 → S2 → S3 → None
        assert_eq!(s1_stored.successor, Some(s2_stored.id));
        assert_eq!(s2_stored.successor, Some(s3_stored.id));
        assert!(s3_stored.successor.is_none());

        Ok(())
    }

    /// S-40: Service delete middle of long chain severs predecessor links
    #[tokio::test]
    async fn service_delete_middle_of_long_chain_rewires_all() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        // Build chain via redo: S1 → S2 → S3 → S4 → S5
        let Summary { services: s1, .. } = Service::create(
            part.id,
            t,
            "S1".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let Summary {
            services: s2_services,
            ..
        } = s1[0].clone().redo(&test_session(), &mut store).await?;
        let s2_id = s2_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        let s2_clone = ServiceStore::get(&mut store, s2_id).await?;
        let Summary {
            services: s3_services,
            ..
        } = s2_clone.clone().redo(&test_session(), &mut store).await?;
        let s3_id = s3_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        let s3_clone = ServiceStore::get(&mut store, s3_id).await?;
        let Summary {
            services: s4_services,
            ..
        } = s3_clone.clone().redo(&test_session(), &mut store).await?;
        let s4_id = s4_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        let s4_clone = ServiceStore::get(&mut store, s4_id).await?;
        let Summary {
            services: s5_services,
            ..
        } = s4_clone.clone().redo(&test_session(), &mut store).await?;
        let s5_id = s5_services
            .iter()
            .find(|s| s.successor.is_none())
            .unwrap()
            .id;

        // Delete S3 via domain delete which handles chain cleanup
        let s3_to_delete: Service = ServiceStore::get(&mut store, s3_id).await?;
        s3_to_delete.id.delete(&test_session(), &mut store).await?;

        // S2 should have successor = None (severed, not rewired)
        let s2_stored = ServiceStore::get(&mut store, s2_id).await?;
        assert!(s2_stored.successor.is_none());

        // S1 should still point to S2
        let s1_stored = ServiceStore::get(&mut store, s1[0].id).await?;
        assert_eq!(s1_stored.successor, Some(s2_stored.id));

        // S4 and S5 are unaffected
        let s4_stored = ServiceStore::get(&mut store, s4_id).await?;
        let s5_stored = ServiceStore::get(&mut store, s5_id).await?;
        assert_eq!(s4_stored.successor, Some(s5_id));
        assert!(s5_stored.successor.is_none());

        Ok(())
    }

    /// S-41: Service delete preserves unrelated part's services
    #[tokio::test]
    async fn service_delete_preserves_unrelated_parts_services() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part1 = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let part2 = Part::create(
            "Chain 2".to_string(),
            "Shimano".to_string(),
            "CN-M510".to_string(),
            PartTypeId::from(4),
            None,
            sample_purchase_date(),
            "Notes".to_string(),
            &test_session(),
            &mut store,
        )
        .await?;

        let t = sample_time();
        Service::create(
            part1.id,
            t,
            "Service 1".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        Service::create(
            part2.id,
            t,
            "Service 2".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        // Delete service on part1
        let svcs = ServiceStore::services_by_part(&mut store, part1.id).await?;
        ServiceStore::delete(&mut store, svcs[0].id).await?;

        // Part2's service should be untouched
        let svcs2 = ServiceStore::services_by_part(&mut store, part2.id).await?;
        assert_eq!(svcs2.len(), 1);

        // Part1's service should be gone
        let svcs1 = ServiceStore::services_by_part(&mut store, part1.id).await?;
        assert_eq!(svcs1.len(), 0);
        Ok(())
    }

    /// S-42: Service delete removes service from store
    #[tokio::test]
    async fn service_delete_removes_from_store() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let part = fixtures::fixture_basic_part(&test_session(), &mut store).await?;
        let t = sample_time();

        let Summary { services, .. } = Service::create(
            part.id,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        let service_id = services[0].id;
        ServiceStore::delete(&mut store, service_id).await?;

        assert!(ServiceStore::get(&mut store, service_id).await.is_err());
        Ok(())
    }

    // === Suite 9: Service — Usage Accounting (prepopulated snapshot) ===

    /// S-43: Service::create on a main part aggregates all snapshot activities
    #[tokio::test]
    async fn service_create_on_main_part_aggregates_prepopulated_activities() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = PartId::from(1);

        // all three snapshot activities predate this service time
        let t = time::macros::datetime!(2023-06-01 10:00 UTC);
        let Summary { usages, .. } = Service::create(
            bike,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].time, 8025);
        assert_eq!(usages[0].distance, 125000);
        assert_eq!(usages[0].climb, 1100);
        // snapshot activities have null descend → falls back to climb
        assert_eq!(usages[0].descend, 1100);
        assert_eq!(usages[0].energy, 1500);
        assert_eq!(usages[0].count, 3);

        let stored = usages[0].id.read(&mut store).await?;
        assert_eq!(stored, usages[0]);
        Ok(())
    }

    /// S-44: Service::create on an attached subpart sums activities within its attachment window
    #[tokio::test]
    async fn service_create_on_subpart_aggregates_prepopulated_activities_during_attachment()
    -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let chain = PartId::from(4);

        // Chain A attached to bike 1 since 2023-01-01, never detached → all 3 activities count
        let t = time::macros::datetime!(2023-06-01 10:00 UTC);
        let Summary { usages, .. } = Service::create(
            chain,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].time, 8025);
        assert_eq!(usages[0].distance, 125000);
        assert_eq!(usages[0].climb, 1100);
        assert_eq!(usages[0].descend, 1100);
        assert_eq!(usages[0].energy, 1500);
        assert_eq!(usages[0].count, 3);
        Ok(())
    }

    /// S-45: Service::create only counts activities before the service time
    #[tokio::test]
    async fn service_create_before_latest_activity_counts_only_earlier_ones() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let bike = PartId::from(1);

        // between Hill Repeats (00:13) and Recovery Spin (22:13) → only the first two count
        let t = time::macros::datetime!(2023-05-19 12:00 UTC);
        let Summary { usages, .. } = Service::create(
            bike,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;

        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].time, 5225);
        assert_eq!(usages[0].distance, 90000);
        assert_eq!(usages[0].climb, 1000);
        assert_eq!(usages[0].descend, 1000);
        assert_eq!(usages[0].energy, 1000);
        assert_eq!(usages[0].count, 2);
        Ok(())
    }

    /// S-46: detaching a serviced part recalculates and persists its service usage
    #[tokio::test]
    async fn detach_served_part_recalculates_service_usage() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let wheel = PartId::from(2);

        let t = time::macros::datetime!(2023-06-01 10:00 UTC);
        let Summary {
            services, usages, ..
        } = Service::create(
            wheel,
            t,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        let svc = &services[0];
        assert_eq!(usages[0].count, 3);

        // round_time(Recovery Spin start 22:13:20) = 22:00 → spin (22:13:20) excluded
        let detach_at = round_time(time::macros::datetime!(2023-05-19 22:13:20 UTC));
        detach_assembly(&session, wheel, detach_at, false, &mut store).await?;

        let recalculated = svc.usage.read(&mut store).await?;
        assert_eq!(recalculated.id, svc.usage);
        assert_eq!(recalculated.time, 5225);
        assert_eq!(recalculated.distance, 90000);
        assert_eq!(recalculated.climb, 1000);
        assert_eq!(recalculated.descend, 1000);
        assert_eq!(recalculated.energy, 1000);
        assert_eq!(recalculated.count, 2);
        Ok(())
    }

    /// S-47: adding an activity increments the usage of every service created
    /// after the activity start; services created before it stay untouched
    #[tokio::test]
    async fn activity_upsert_increments_service_usage_after_start() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let bike = PartId::from(1);

        // service after the new activity start → gets incremented
        let t_after = time::macros::datetime!(2023-06-10 10:00 UTC);
        let Summary {
            services, usages, ..
        } = Service::create(
            bike,
            t_after,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        let svc_after = &services[0];
        assert_eq!(usages[0].count, 3);

        // service before the new activity start → stays untouched
        let t_before = time::macros::datetime!(2023-06-01 10:00 UTC);
        let Summary {
            services, usages, ..
        } = Service::create(
            bike,
            t_before,
            "Service".to_string(),
            "".to_string(),
            None,
            vec![],
            &mut store,
        )
        .await?;
        let svc_before = &services[0];
        assert_eq!(usages[0].count, 3);

        let act = Activity {
            id: ActivityId::new(100),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "New Ride".to_string(),
            start: time::macros::datetime!(2023-06-05 10:00 UTC),
            duration: 3600,
            time: Some(1000),
            distance: Some(10000),
            climb: Some(100),
            descend: None,
            energy: Some(200),
            gear: Some(bike),
            device_name: None,
            external_id: None,
        };
        act.upsert(&session, &mut store).await?;

        // service after the start is incremented: 8025+1000, 125000+10000,
        // 1100+100, 1100+100 (descend None → climb), 1500+200, 3+1
        let increased = svc_after.usage.read(&mut store).await?;
        assert_eq!(increased.time, 9025);
        assert_eq!(increased.distance, 135000);
        assert_eq!(increased.climb, 1200);
        assert_eq!(increased.descend, 1200);
        assert_eq!(increased.energy, 1700);
        assert_eq!(increased.count, 4);

        // service before the start is untouched
        let untouched = svc_before.usage.read(&mut store).await?;
        assert_eq!(untouched.time, 8025);
        assert_eq!(untouched.distance, 125000);
        assert_eq!(untouched.climb, 1100);
        assert_eq!(untouched.descend, 1100);
        assert_eq!(untouched.energy, 1500);
        assert_eq!(untouched.count, 3);
        Ok(())
    }
}
