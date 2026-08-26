use super::*;
use crate::{PartId, Service, ServiceId, TbResult};

#[async_trait::async_trait]
impl ServiceStore for MemStore {
    async fn create(&mut self, _service: Service) -> TbResult<Service> {
        todo!()
    }

    async fn get(&mut self, _service: ServiceId) -> TbResult<Service> {
        todo!()
    }

    async fn update(&mut self, _service: Service) -> TbResult<Service> {
        todo!()
    }

    async fn delete(&mut self, _service: ServiceId) -> TbResult<usize> {
        todo!()
    }

    async fn services_delete(&mut self, _services: &[Service]) -> TbResult<usize> {
        todo!()
    }

    async fn services_by_part(&mut self, _part: PartId) -> TbResult<Vec<Service>> {
        Ok(self.services.values().cloned().collect())
    }
}
