use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use tb_domain::{schema, PartId, Service, ServiceId, TbResult};

use crate::{map_to_tb, AsyncDieselConn};
use schema::services::dsl::*;

#[async_session::async_trait]
impl tb_domain::ServiceStore for AsyncDieselConn {
    async fn create(&mut self, service: &Service) -> TbResult<Service> {
        diesel::insert_into(services)
            .values(service)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        services
            .find(service)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        diesel::update(services.find(service.id))
            .set(service)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        services
            .filter(part_id.eq(part))
            .get_results(self)
            .await
            .map_err(map_to_tb)
    }
}
