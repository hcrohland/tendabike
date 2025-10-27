use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::schema;
use crate::{AsyncDieselConn, into_domain, vec_into};
use ::time::OffsetDateTime;
use schema::services::table;
use tb_domain::{PartId, Service, ServiceId, TbResult};
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, PartialEq, Eq, Identifiable, Insertable, AsChangeset)]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = schema::services)]
pub struct DbService {
    pub id: Uuid,
    /// the part serviced
    part_id: i32,
    /// when it was serviced
    time: OffsetDateTime,
    /// when there was a new service
    redone: OffsetDateTime,
    // we do not accept theses values from the client!
    name: String,
    notes: String,
    // we do not accept theses values from the client!
    usage: Uuid,
    // the predecessor Service
    successor: Option<Uuid>,
    // an optional ServicePlan it is fullfilling
    plans: Vec<Uuid>,
}

impl From<Service> for DbService {
    fn from(value: Service) -> Self {
        let Service {
            id,
            part_id,
            time,
            redone,
            name,
            notes,
            usage,
            successor,
            plans,
        } = value;
        DbService {
            id: id.into(),
            part_id: part_id.into(),
            time,
            redone,
            name,
            notes,
            usage: usage.into(),
            successor: successor.map(Into::into),
            plans: vec_into(plans),
        }
    }
}

impl From<DbService> for Service {
    fn from(value: DbService) -> Self {
        let DbService {
            id,
            part_id,
            time,
            redone,
            name,
            notes,
            usage,
            successor,
            plans,
        } = value;
        Service {
            id: id.into(),
            part_id: part_id.into(),
            time,
            redone,
            name,
            notes,
            usage: usage.into(),
            successor: successor.map(Into::into),
            plans: vec_into(plans),
        }
    }
}

#[async_session::async_trait]
impl tb_domain::ServiceStore for AsyncDieselConn {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        let service: DbService = service.into();
        diesel::insert_into(table)
            .values(service)
            .get_result::<DbService>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        let service: Uuid = service.into();
        table
            .find(service)
            .get_result::<DbService>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        let service: DbService = service.into();
        diesel::update(table.find(service.id))
            .set(service)
            .get_result::<DbService>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> {
        diesel::delete(table.find(Uuid::from(service)))
            .execute(self)
            .await
            .map_err(into_domain)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        table
            .filter(schema::services::part_id.eq(i32::from(part)))
            .get_results::<DbService>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn services_delete(&mut self, list: &[Service]) -> TbResult<usize> {
        use schema::services::dsl::*;
        let list: Vec<_> = list.iter().map(|s| Uuid::from(s.id)).collect();

        diesel::delete(services.filter(id.eq_any(list)))
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
