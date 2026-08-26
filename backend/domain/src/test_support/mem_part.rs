use super::*;
use crate::{Error, Part, PartId, PartStore, PartTypeId, ShopId, TbResult, UsageId, UserId};
use async_trait::async_trait;
use time::OffsetDateTime;

#[async_trait]
impl PartStore for MemStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        self.parts
            .get(&pid)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Part {} not found", pid)))
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        Ok(self
            .parts
            .values()
            .filter(|p| &p.owner == uid)
            .cloned()
            .collect())
    }

    async fn part_create(
        &mut self,
        what: PartTypeId,
        name: String,
        vendor: String,
        model: String,
        purchase: OffsetDateTime,
        source: Option<String>,
        notes: String,
        usage: UsageId,
        owner: UserId,
        shop: Option<ShopId>,
    ) -> TbResult<Part> {
        let id = PartId::from(self.next_part_id);
        self.next_part_id += 1;
        let part = Part {
            id: id.clone(),
            owner,
            what,
            name,
            vendor,
            model,
            purchase,
            last_used: purchase,
            disposed_at: None,
            usage,
            source,
            notes,
            shop,
        };
        self.parts.insert(id, part.clone());
        Ok(part)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        match self.parts.insert(part.id.clone(), part.clone()) {
            Some(_) => Ok(part),
            None => Err(Error::NotFound(format!("Part {} not found", part.id))),
        }
    }

    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> {
        match self.parts.remove(&part) {
            Some(_) => Ok(part),
            None => Err(Error::NotFound(format!("Part {} not found", part))),
        }
    }

    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> {
        let mut count = 0;
        for part in parts {
            if self.parts.remove(&part.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        Ok(self
            .parts
            .values()
            .find(|p| p.source.as_deref() == Some(strava_id))
            .map(|p| p.id.clone()))
    }

    async fn parts_register_shop(
        &mut self,
        shop_id: ShopId,
        part_ids: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        let mut result = Vec::new();
        for pid in part_ids {
            if let Some(part) = self.parts.get_mut(&pid) {
                part.shop = Some(shop_id.clone());
                result.push(part.clone());
            }
        }
        Ok(result)
    }

    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        let mut result = Vec::new();
        for pid in part_ids {
            if let Some(part) = self.parts.get_mut(&pid) {
                part.shop = None;
                result.push(part.clone());
            }
        }
        Ok(result)
    }

    async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> {
        Ok(self
            .parts
            .values()
            .filter(|p| p.shop.as_ref() == Some(&shop_id))
            .cloned()
            .collect())
    }
}
