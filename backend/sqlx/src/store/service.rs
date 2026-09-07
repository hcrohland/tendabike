use crate::{SqlxConn, into_domain, vec_into};
use ::time::OffsetDateTime;
use sqlx::FromRow;
use tb_domain::{PartId, Service, ServiceId, TbResult};
use uuid::Uuid;

#[derive(Clone, Debug, FromRow, PartialEq, Eq)]
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

#[async_trait::async_trait]
impl<'c> tb_domain::ServiceStore for SqlxConn<'c> {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        let service: DbService = service.into();
        sqlx::query_as!(
            DbService,
            r#"INSERT INTO services (id, part_id, time, redone, name, notes, usage, successor, plans)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, part_id, time, redone, name, notes, usage, successor, plans as "plans!""#,
            service.id,
            service.part_id,
            service.time,
            service.redone,
            service.name,
            service.notes,
            service.usage,
            service.successor,
            &service.plans as _
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        sqlx::query_as!(
            DbService,
            r#"SELECT id, part_id, time, redone, name, notes, usage, successor, plans as "plans!" FROM services WHERE id = $1"#,
            Uuid::from(service)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        let service: DbService = service.into();
        sqlx::query_as!(
            DbService,
            r#"UPDATE services
             SET part_id = $2, time = $3, redone = $4, name = $5, notes = $6,
                 usage = $7, successor = $8, plans = $9
             WHERE id = $1
             RETURNING id, part_id, time, redone, name, notes, usage, successor, plans as "plans!""#,
            service.id,
            service.part_id,
            service.time,
            service.redone,
            service.name,
            service.notes,
            service.usage,
            service.successor,
            &service.plans as _
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> {
        let result = sqlx::query!("DELETE FROM services WHERE id = $1", Uuid::from(service))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        sqlx::query_as!(
            DbService,
            r#"SELECT id, part_id, time, redone, name, notes, usage, successor, plans as "plans!" FROM services WHERE part_id = $1"#,
            i32::from(part)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn services_delete(&mut self, list: &[Service]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| Uuid::from(s.id)).collect();

        let result = sqlx::query!("DELETE FROM services WHERE id = ANY($1)", &list as _)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tb_domain::{ServicePlanId, UsageId};

    fn date() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
    }

    fn uuid() -> Uuid {
        Uuid::from_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap()
    }

    fn service() -> Service {
        Service {
            id: ServiceId::from(uuid()),
            part_id: PartId::from(2),
            time: date(),
            redone: date(),
            name: "Bleed brakes".to_string(),
            notes: "all four".to_string(),
            usage: UsageId::from(uuid()),
            successor: Some(ServiceId::from(uuid())),
            plans: vec![ServicePlanId::from(uuid())],
        }
    }

    #[test]
    fn service_into_db_maps_ids_and_plans() {
        let service = service();
        let db = DbService::from(service.clone());
        assert_eq!(db.id, Uuid::from(service.id));
        assert_eq!(db.part_id, 2);
        assert_eq!(db.usage, Uuid::from(service.usage));
        assert_eq!(db.successor, service.successor.map(Uuid::from));
        assert_eq!(
            db.plans,
            service
                .plans
                .iter()
                .copied()
                .map(Uuid::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn service_db_roundtrip_preserves_fields() {
        let service = service();
        assert_eq!(Service::from(DbService::from(service.clone())), service);
    }
}
