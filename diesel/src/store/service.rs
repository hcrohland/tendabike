use tb_domain::{PartId, Service, ServiceId, TbResult};

use crate::AsyncDieselConn;

#[async_session::async_trait]
impl tb_domain::ServiceStore for AsyncDieselConn {
    async fn create(&mut self, service: &Service) -> TbResult<Service> {
        todo!()
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        todo!()
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        todo!()
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        todo!()
    }
}
