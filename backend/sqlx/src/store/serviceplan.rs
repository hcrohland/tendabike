use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use tb_domain::{PartId, ServicePlan, ServicePlanId, TbResult, UserId};
use uuid::Uuid;

use super::schema;
use crate::{AsyncDieselConn, into_domain, vec_into};
use schema::service_plans::table;

#[derive(Clone, Debug, PartialEq, Queryable, Identifiable, Insertable, AsChangeset)]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = schema::service_plans)]
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
impl tb_domain::ServicePlanStore for AsyncDieselConn {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        diesel::insert_into(table)
            .values(DbServicePlan::from(plan))
            .get_result::<DbServicePlan>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        table
            .find(Uuid::from(plan))
            .get_result::<DbServicePlan>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        let plan: DbServicePlan = plan.into();
        diesel::update(table.find(plan.id))
            .set(plan)
            .get_result::<DbServicePlan>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        diesel::delete(table.find(Uuid::from(plan)))
            .execute(self)
            .await
            .map_err(into_domain)
    }

    async fn by_part(&mut self, part_id: PartId) -> TbResult<Vec<ServicePlan>> {
        table
            .filter(schema::service_plans::part.eq(i32::from(part_id)))
            .get_results::<DbServicePlan>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn by_user(&mut self, user_id: UserId) -> TbResult<Vec<ServicePlan>> {
        table
            .filter(schema::service_plans::uid.eq(i32::from(user_id)))
            .get_results::<DbServicePlan>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn serviceplans_delete(&mut self, plans: &[ServicePlan]) -> TbResult<usize> {
        use schema::service_plans::dsl::*;

        let plans: Vec<_> = plans.iter().map(|s| Uuid::from(s.id)).collect();

        diesel::delete(service_plans.filter(id.eq_any(plans)))
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
