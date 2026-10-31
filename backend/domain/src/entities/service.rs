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
        ServiceStore::service_get(store, self).await
    }

    pub async fn delete(
        self,
        user: &dyn Session,
        store: &mut (impl PartStore + ServiceStore + ShopStore + UsageStore),
    ) -> TbResult<Summary> {
        let service = self.get(store).await?;
        service.part_id.checkuser(user, store).await?;

        // find predecessors
        let services = store
            .services_by_part(service.part_id)
            .await?
            .into_iter()
            .filter(|s| s.successor == Some(service.id));

        // set successors to none
        let mut summary = Summary::default();
        for mut s in services {
            s.successor = None;
            let s = ServiceStore::service_update(store, s).await?;
            summary.services.upsert(s);
        }

        // delete service: it is reported as a None tombstone, so the
        // client's merge drops its row (issue #462)
        service.usage.delete(store).await?;
        ServiceStore::service_delete(store, self).await?;
        summary.services.tombstone(service.key());
        Ok(summary)
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

impl IdKeyed for Service {
    type Key = ServiceId;
    fn key(&self) -> Self::Key {
        self.id
    }
}

impl Service {
    /// Deletes a specific set of services. The user-deletion cascade
    /// (ADR-0010) uses this: the caller already owns these services.
    pub(crate) async fn delete_many(
        services: &[Service],
        store: &mut impl ServiceStore,
    ) -> TbResult<usize> {
        store.services_delete(services).await
    }

    /// The shared creation logic: no ownership check. The public [`Service::create`]
    /// and [`Service::redo`] check the part's ownership at their entry point and
    /// delegate here — the guard lives at the public op (ADR-0008), the creation
    /// logic is one private unit.
    async fn create_unchecked(
        part_id: PartId,
        time: OffsetDateTime,
        name: String,
        notes: String,
        successor: Option<ServiceId>,
        plans: Vec<ServicePlanId>,
        store: &mut (impl ActivityStore + AttachmentStore + PartStore + ServiceStore + UsageStore),
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
        let service = ServiceStore::service_create(store, service).await?;
        let mut summary = Summary::default();
        summary.services.upsert(service);
        summary.usages.upsert(usage);
        Ok(summary)
    }

    /// Creates a service on the part, checking the part's ownership first — the
    /// guard the dispatch used to run, now at this public entry point (like
    /// [`Service::redo`] and [`Service::update`]).
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        part_id: PartId,
        time: OffsetDateTime,
        name: String,
        notes: String,
        successor: Option<ServiceId>,
        plans: Vec<ServicePlanId>,
        user: &dyn Session,
        store: &mut (
                 impl ActivityStore + AttachmentStore + PartStore + ServiceStore + ShopStore + UsageStore
             ),
    ) -> TbResult<Summary> {
        part_id.checkuser(user, store).await?;
        Self::create_unchecked(part_id, time, name, notes, successor, plans, store).await
    }

    async fn calculate_usage(
        &self,
        store: &mut (impl ActivityStore + AttachmentStore + PartStore),
    ) -> TbResult<Usage> {
        Ok(if self.part_id.is_main(store).await? {
            Activity::find(self.part_id, MIN_TIME, self.time, store).await?
        } else {
            Attachment::activities_by_part(self.part_id, MIN_TIME, self.time, store).await?
        }
        .into_iter()
        .fold(Usage::new(self.usage), |usage, act| usage + &act.usage()))
    }

    pub async fn redo(
        self,
        user: &dyn Session,
        store: &mut (
                 impl ActivityStore + AttachmentStore + PartStore + ServiceStore + ShopStore + UsageStore
             ),
    ) -> TbResult<Summary> {
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
            Self::create_unchecked(
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
            let mut res = Self::create_unchecked(
                old.part_id,
                time,
                old.name.clone(),
                notes,
                None,
                plans,
                store,
            )
            .await?;
            let new_id = res
                .services
                .live()
                .pop()
                .expect("create reports the new service")
                .id;
            old.successor = Some(new_id);
            res += old.update_unchecked(store).await?;
            Ok(res)
        }
    }

    async fn update_unchecked(
        self,
        store: &mut (impl ActivityStore + AttachmentStore + PartStore + ServiceStore + UsageStore),
    ) -> TbResult<Summary> {
        let usages = vec![self.calculate_usage(store).await?.update(store).await?];
        let services = vec![ServiceStore::service_update(store, self).await?];
        let mut summary = Summary::default();
        summary.usages.upsert_all(usages);
        summary.services.upsert_all(services);
        Ok(summary)
    }

    pub async fn update(
        mut self,
        user: &dyn Session,
        store: &mut (
                 impl ActivityStore + AttachmentStore + PartStore + ServiceStore + ShopStore + UsageStore
             ),
    ) -> TbResult<Summary> {
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
        store: &mut (impl ActivityStore + AttachmentStore + PartStore + ServiceStore),
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
        store: &mut (impl ServiceStore + UsageStore),
    ) -> TbResult<(Vec<Service>, Vec<Usage>)> {
        let services = store.services_by_part(part).await?;

        let mut usages = Vec::new();
        for serv in &services {
            usages.push(serv.usage.read(store).await?);
        }
        Ok((services, usages))
    }

    pub(crate) async fn unlink_plan(
        plan: ServicePlanId,
        owner: UserId,
        store: &mut (impl PartStore + ServiceStore),
    ) -> TbResult<Vec<Service>> {
        let mut res = Vec::new();
        for part in Part::get_all(&owner, store).await? {
            for mut service in store.services_by_part(part.id).await? {
                if service.plans.contains(&plan) {
                    service.plans.retain(|p| *p != plan);
                    res.push(store.service_update(service).await?);
                }
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod service_tests;
