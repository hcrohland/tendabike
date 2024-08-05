use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use tb_domain::{schema, PartId, Service, ServiceId, TbResult};

use crate::{into_domain, AsyncDieselConn};
use schema::services::dsl::*;

#[async_session::async_trait]
impl tb_domain::ServiceStore for AsyncDieselConn {
    async fn create(&mut self, service: &Service) -> TbResult<Service> {
        diesel::insert_into(services)
            .values(service)
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        services
            .find(service)
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        diesel::update(services.find(service.id))
            .set(service)
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> {
        diesel::delete(services.find(service))
            .execute(self)
            .await
            .map_err(into_domain)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        services
            .filter(part_id.eq(part))
            .get_results(self)
            .await
            .map_err(into_domain)
    }
}
