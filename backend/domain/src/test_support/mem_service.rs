use super::*;
use crate::{PartId, Service, ServiceId, TbResult};

#[async_trait::async_trait]
impl ServiceStore for MemStore {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        todo!()
    }

    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        todo!()
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        todo!()
    }

    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> {
        todo!()
    }

    async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize> {
        todo!()
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        todo!()
    }
}
