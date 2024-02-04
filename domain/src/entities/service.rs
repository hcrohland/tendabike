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

    async fn get(self, store: &mut impl ServiceStore) -> TbResult<Service> {
        ServiceStore::get(store, self).await
    }

    pub async fn delete(self, user: &dyn Person, store: &mut impl Store) -> TbResult<usize> {
        let service = self.get(store).await?;
        service.part_id.checkuser(user, store).await?;
        service.usage.delete(store).await?;
        ServiceStore::delete(store, self).await
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
        redone: Option<OffsetDateTime>,
        name: String,
        notes: String,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let service = Service {
            id: ServiceId::new(),
            part_id,
            time,
            redone: redone.unwrap_or(MAX_TIME),
            name,
            notes,
            usage: UsageId::new(),
        };
        let usage = service.calculate_usage(store).await?.update(store).await?;
        let service = ServiceStore::create(store, &service).await?;
        Ok(Summary {
            services: vec![service],
            usages: vec![usage],
            ..Default::default()
        })
    }

    async fn calculate_usage(&self, store: &mut impl Store) -> TbResult<Usage> {
        Ok(if self.part_id.is_main(store).await? {
            Activity::find(self.part_id, self.time, self.redone, store).await?
        } else {
            Attachment::activities_by_part(self.part_id, self.time, self.redone, store).await?
        }
        .into_iter()
        .fold(Usage::new(self.usage), |usage, act| usage + &act.usage()))
    }

    pub async fn redo(
        id: ServiceId,
        time: OffsetDateTime,
        notes: String,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let mut old = id.get(store).await?;
        old.part_id.checkuser(user, store).await?;
        Ok(if time < old.time {
            Service::create(old.part_id, time, Some(old.time), old.name, notes, store).await?
        } else {
            old.redone = time;
            Service::create(old.part_id, time, None, old.name.clone(), notes, store).await?
                + old.update_unchecked(store).await?
        })
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

    pub async fn update(mut self, user: &dyn Person, store: &mut impl Store) -> TbResult<Summary> {
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
