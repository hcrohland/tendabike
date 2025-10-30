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

#[async_session::async_trait]
impl tb_domain::ServiceStore for SqlxConn {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        let service: DbService = service.into();
        sqlx::query_as::<_, DbService>(
            "INSERT INTO services (id, part_id, time, redone, name, notes, usage, successor, plans)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING *",
        )
        .bind(service.id)
        .bind(service.part_id)
        .bind(service.time)
        .bind(service.redone)
        .bind(service.name)
        .bind(service.notes)
        .bind(service.usage)
        .bind(service.successor)
        .bind(service.plans)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        sqlx::query_as::<_, DbService>("SELECT * FROM services WHERE id = $1")
            .bind(Uuid::from(service))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        let service: DbService = service.into();
        sqlx::query_as::<_, DbService>(
            "UPDATE services
             SET part_id = $2, time = $3, redone = $4, name = $5, notes = $6,
                 usage = $7, successor = $8, plans = $9
             WHERE id = $1
             RETURNING *",
        )
        .bind(service.id)
        .bind(service.part_id)
        .bind(service.time)
        .bind(service.redone)
        .bind(service.name)
        .bind(service.notes)
        .bind(service.usage)
        .bind(service.successor)
        .bind(service.plans)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> {
        let result = sqlx::query("DELETE FROM services WHERE id = $1")
            .bind(Uuid::from(service))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        sqlx::query_as::<_, DbService>("SELECT * FROM services WHERE part_id = $1")
            .bind(i32::from(part))
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn services_delete(&mut self, list: &[Service]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| Uuid::from(s.id)).collect();

        let result = sqlx::query("DELETE FROM services WHERE id = ANY($1)")
            .bind(&list)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
