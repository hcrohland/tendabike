use sqlx::FromRow;

use tb_domain::{PartId, ServicePlan, ServicePlanId, TbResult, UserId};
use uuid::Uuid;

use crate::{SqlxConn, into_domain, vec_into};

#[derive(Clone, Debug, PartialEq, FromRow)]
struct DbServicePlan {
    pub id: Uuid,
    pub part: Option<i32>,
    what: i32,
    hook: Option<i32>,
    name: String,
    pub days: Option<i32>,
    pub hours: Option<i32>,
    pub km: Option<i32>,
    pub climb: Option<i32>,
    pub descend: Option<i32>,
    pub rides: Option<i32>,
    pub uid: Option<i32>,
    pub energy: Option<i32>,
}

impl From<ServicePlan> for DbServicePlan {
    fn from(value: ServicePlan) -> Self {
        let ServicePlan {
            id,
            part,
            what,
            hook,
            name,
            days,
            hours,
            km,
            climb,
            descend,
            rides,
            uid,
            energy,
        } = value;
        Self {
            id: id.into(),
            part: part.map(Into::into),
            what: what.into(),
            hook: hook.map(Into::into),
            name,
            days,
            hours,
            km,
            climb,
            descend,
            rides,
            uid: uid.map(Into::into),
            energy,
        }
    }
}

impl From<DbServicePlan> for ServicePlan {
    fn from(value: DbServicePlan) -> Self {
        let DbServicePlan {
            id,
            part,
            what,
            hook,
            name,
            days,
            hours,
            km,
            climb,
            descend,
            rides,
            uid,
            energy,
        } = value;
        Self {
            id: id.into(),
            part: part.map(Into::into),
            what: what.into(),
            hook: hook.map(Into::into),
            name,
            days,
            hours,
            km,
            climb,
            descend,
            rides,
            uid: uid.map(Into::into),
            energy,
        }
    }
}

#[async_session::async_trait]
impl tb_domain::ServicePlanStore for SqlxConn {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let plan = DbServicePlan::from(plan);
        sqlx::query_as::<_, DbServicePlan>(
            "INSERT INTO service_plans (id, part, what, hook, name, days, hours, km, climb, descend, rides, uid, energy)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             RETURNING *"
        )
        .bind(plan.id)
        .bind(plan.part)
        .bind(plan.what)
        .bind(plan.hook)
        .bind(plan.name)
        .bind(plan.days)
        .bind(plan.hours)
        .bind(plan.km)
        .bind(plan.climb)
        .bind(plan.descend)
        .bind(plan.rides)
        .bind(plan.uid)
        .bind(plan.energy)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        sqlx::query_as::<_, DbServicePlan>("SELECT * FROM service_plans WHERE id = $1")
            .bind(Uuid::from(plan))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let plan: DbServicePlan = plan.into();
        sqlx::query_as::<_, DbServicePlan>(
            "UPDATE service_plans
             SET part = $2, what = $3, hook = $4, name = $5, days = $6, hours = $7,
                 km = $8, climb = $9, descend = $10, rides = $11, uid = $12, energy = $13
             WHERE id = $1
             RETURNING *",
        )
        .bind(plan.id)
        .bind(plan.part)
        .bind(plan.what)
        .bind(plan.hook)
        .bind(plan.name)
        .bind(plan.days)
        .bind(plan.hours)
        .bind(plan.km)
        .bind(plan.climb)
        .bind(plan.descend)
        .bind(plan.rides)
        .bind(plan.uid)
        .bind(plan.energy)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        let result = sqlx::query("DELETE FROM service_plans WHERE id = $1")
            .bind(Uuid::from(plan))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn by_part(&mut self, part_id: PartId) -> TbResult<Vec<ServicePlan>> {
        sqlx::query_as::<_, DbServicePlan>("SELECT * FROM service_plans WHERE part = $1")
            .bind(i32::from(part_id))
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn by_user(&mut self, user_id: UserId) -> TbResult<Vec<ServicePlan>> {
        sqlx::query_as::<_, DbServicePlan>("SELECT * FROM service_plans WHERE uid = $1")
            .bind(i32::from(user_id))
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn serviceplans_delete(&mut self, plans: &[ServicePlan]) -> TbResult<usize> {
        let plans: Vec<_> = plans.iter().map(|s| Uuid::from(s.id)).collect();

        let result = sqlx::query("DELETE FROM service_plans WHERE id = ANY($1)")
            .bind(&plans)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
