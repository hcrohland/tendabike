use crate::{PartId, Service, ServiceId, TbResult};

#[async_trait::async_trait]
/// A trait representing a store for `Usage` objects.
pub trait ServiceStore {
    async fn create(&mut self, service: &Service) -> TbResult<Service>;
    async fn get(&mut self, service: ServiceId) -> TbResult<Service>;
    async fn update(&mut self, service: Service) -> TbResult<Service>;
    async fn delete(&mut self, service: ServiceId) -> TbResult<usize>;
    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>>;
}
