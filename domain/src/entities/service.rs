use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::*;
use schema::services;

#[derive(
    DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub struct ServiceId(Uuid);

NewtypeDisplay! { () pub struct ServiceId(); }
NewtypeFrom! { () pub struct ServiceId(Uuid); }

impl ServiceId {
    pub(crate) fn new() -> Self {
        Uuid::now_v7().into()
    }

    async fn finish(self, time: OffsetDateTime, store: &mut impl Store) -> TbResult<Service> {
        let mut service = ServiceStore::get(store, self).await?;
        service.redone = time;
        service.calculate_usage(store).await?.update(store).await?;
        ServiceStore::update(store, service).await
    }
}

/// Timeline of attachments
///
/// * Every attachment of a part to a specified hook on a gear is an entry
/// * Start and end time are noted
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
pub struct Service {
    pub id: ServiceId,
    /// the part serviced
    part_id: PartId,
    /// when it was serviced
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    /// when there was a new service
    #[serde(with = "time::serde::rfc3339")]
    redone: OffsetDateTime,
    // we do not accept theses values from the client!
    name: String,
    notes: String,
    usage: UsageId,
}

impl Service {
    pub async fn create(
        part_id: PartId,
        time: OffsetDateTime,
        name: String,
        notes: String,
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
        };
        let usage = service.calculate_usage(store).await?;
        usage.update(store).await?;
        let service = ServiceStore::create(store, &service).await?;
        let usage = service.usage.read(store).await?;
        Ok(Summary {
            services: vec![service],
            usages: vec![usage],
            ..Default::default()
        })
    }

    async fn calculate_usage(&self, store: &mut impl Store) -> TbResult<Usage> {
        Ok(
            Attachment::activities_by_part(self.part_id, self.time, self.redone, store)
                .await?
                .into_iter()
                .fold(Usage::new(self.usage), |usage, act| usage + &act.usage()),
        )
    }

    pub async fn redo(
        id: ServiceId,
        time: OffsetDateTime,
        notes: String,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let old = id.finish(time, store).await?;
        Service::create(old.part_id, time, old.name, notes, store).await?;
        todo!("collect old results")
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
            .filter(|s: &Service| s.time <= time && s.redone > time)
            .map(|s| s.usage)
            .collect())
    }

    pub(crate) async fn recalculate(
        part: PartId,
        start: OffsetDateTime,
        end: OffsetDateTime,
        store: &mut impl Store,
    ) -> TbResult<Vec<Usage>> {
        let mut res = Vec::new();
        let services = store
            .services_by_part(part)
            .await?
            .into_iter()
            .filter(|s: &Service| s.time <= end && s.redone > start);
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
}
