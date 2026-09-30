use super::*;
use crate::{PartId, Service, ServiceId, TbResult};

#[async_trait::async_trait]
impl ServiceStore for MemStore {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        let d = self.state_mut();
        d.services.insert(service.id, service.clone());
        Ok(service)
    }

    async fn get(&mut self, id: ServiceId) -> TbResult<Service> {
        let d = self.state();
        d.services
            .get(&id)
            .cloned()
            .ok_or(crate::Error::NotFound(format!("Service {} not found", id)))
    }

    async fn update(&mut self, service: Service) -> TbResult<Service> {
        let d = self.state_mut();
        match d.services.get_mut(&service.id) {
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
        let d = self.state_mut();
        match d.services.remove(&id) {
            Some(_) => Ok(1),
            None => Ok(0),
        }
    }

    async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize> {
        let d = self.state_mut();
        let mut count = 0;
        for s in services {
            if d.services.remove(&s.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        let d = self.state();
        Ok(d.services
            .values()
            .filter(|s| s.part_id == part)
            .cloned()
            .collect())
    }
}
