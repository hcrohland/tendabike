use super::*;
use crate::{Part, PartId, PartTypeId, ShopId, TbResult, UsageId, UserId};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl PartStore for MemStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        todo!()
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        todo!()
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
        todo!()
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        todo!()
    }

    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> {
        todo!()
    }

    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> {
        todo!()
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        todo!()
    }

    async fn parts_register_shop(
        &mut self,
        shop_id: ShopId,
        part_id: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        todo!()
    }

    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        todo!()
    }

    async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> {
        todo!()
    }
}
