use super::*;
use crate::{PartId, Service, ServiceId, TbResult};

#[async_trait::async_trait]
impl ServiceStore for MemStore {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        self.services.insert(service.id, service.clone());
        Ok(service)
    }

    async fn get(&mut self, id: ServiceId) -> TbResult<Service> {
        self.services
            .get(&id)
            .cloned()
            .ok_or(crate::Error::NotFound(format!("Service {} not found", id)))
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        match self.services.get_mut(&service.id) {
            Some(s) => {
                *s = service.clone();
                Ok(service)
            }
            None => Err(crate::Error::NotFound(format!(
                "Service {} not found",
                service.id
            ))),
        }
    }

    async fn delete(&mut self, id: ServiceId) -> TbResult<usize> {
        match self.services.remove(&id) {
            Some(_) => Ok(1),
            None => Ok(0),
        }
    }

    async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize> {
        let mut count = 0;
        for s in services {
            if self.services.remove(&s.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        Ok(self
            .services
            .values()
            .filter(|s| s.part_id == part)
            .cloned()
            .collect())
    }
}
