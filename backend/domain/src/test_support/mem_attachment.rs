use super::*;
use crate::{Attachment, PartId, PartTypeId, TbResult};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl AttachmentStore for MemStore {
    async fn attachment_create(&mut self, _att: Attachment) -> TbResult<Attachment> {
        todo!()
    }

    async fn delete(&mut self, _att: Attachment) -> TbResult<Attachment> {
        todo!()
    }

    async fn attachments_delete_by_parts(&mut self, _parts: &[crate::Part]) -> TbResult<usize> {
        todo!()
    }

    async fn attachment_get_by_gear_and_time(
        &mut self,
        _act_gear: PartId,
        _start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        todo!()
    }

    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        Ok(self
            .attachments
            .values()
            .filter(|a| a.part_id == id)
            .cloned()
            .collect())
    }

    async fn attachment_get_by_part_and_time(
        &mut self,
        _pid: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        todo!()
    }

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        _types: Vec<crate::PartTypeId>,
        _gear: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        todo!()
    }

    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        _what: PartTypeId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        todo!()
    }

    async fn attachment_find_successor(
        &mut self,
        _part_id: PartId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
        _what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        todo!()
    }

    async fn attachment_find_later_attachment_for_part(
        &mut self,
        _part_id: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        todo!()
    }

    async fn attachment_find_part_attached_already(
        &mut self,
        _part_id: PartId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        todo!()
    }
}
