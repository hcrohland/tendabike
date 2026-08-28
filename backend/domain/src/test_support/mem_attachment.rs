use super::*;
use crate::{Attachment, PartId, PartTypeId, TbResult};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl AttachmentStore for MemStore {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        let key = (att.part_id, att.attached, self.attachment_counter);
        self.attachment_counter += 1;
        self.attachments.insert(key, att);
        Ok(att)
    }

    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        // Find and remove by part_id, attached time, gear, and hook
        let entries: Vec<_> = self
            .attachments
            .iter()
            .filter(|((pid, attached, _), a)| {
                *pid == att.part_id
                    && *attached == att.attached
                    && a.gear == att.gear
                    && a.hook == att.hook
            })
            .map(|(k, _)| k.clone())
            .collect();
        for key in entries {
            if let Some(a) = self.attachments.remove(&key) {
                return Ok(a);
            }
        }
        Err(crate::Error::NotFound(format!(
            "Attachment {} at {:?} not found",
            att.part_id, att.attached
        )))
    }

    async fn attachments_delete_by_parts(&mut self, parts: &[crate::Part]) -> TbResult<usize> {
        let part_ids: Vec<PartId> = parts.iter().map(|p| p.id).collect();
        let before_count = self.attachments.len();
        self.attachments
            .retain(|(pid, _, _), _| !part_ids.contains(pid));
        Ok(before_count - self.attachments.len())
    }

    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        Ok(self
            .attachments
            .values()
            .filter(|a| a.gear == act_gear && a.attached <= start && a.detached > start)
            .cloned()
            .collect())
    }

    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        let mut result: Vec<Attachment> = self
            .attachments
            .values()
            .filter(|a| a.part_id == id)
            .cloned()
            .collect();
        result.sort_by_key(|a| a.attached);
        Ok(result)
    }

    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        Ok(self
            .attachments
            .values()
            .find(|a| a.part_id == pid && a.attached <= time && a.detached > time)
            .cloned())
    }

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        _types: Vec<PartTypeId>,
        gear: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        Ok(self
            .attachments
            .values()
            .filter(|a| a.gear == gear && a.attached <= time && a.detached > time)
            .cloned()
            .collect())
    }

    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        _what: PartTypeId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        Ok(self
            .attachments
            .values()
            .find(|a| a.gear == gear && a.hook == hook && a.attached <= time && a.detached > time)
            .cloned())
    }

    async fn attachment_find_successor(
        &mut self,
        part_id: PartId,
        gear: PartId,
        _hook: PartTypeId,
        time: OffsetDateTime,
        _what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        Ok(self
            .attachments
            .values()
            .filter(|a| a.part_id == part_id && a.gear == gear && a.attached > time)
            .min_by_key(|a| a.attached)
            .cloned())
    }

    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        Ok(self
            .attachments
            .values()
            .filter(|a| a.part_id == part_id && a.attached > time)
            .min_by_key(|a| a.attached)
            .cloned())
    }

    async fn attachment_find_part_attached_already(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        Ok(self
            .attachments
            .values()
            .find(|a| {
                a.part_id == part_id
                    && a.gear == gear
                    && a.hook == hook
                    && a.attached <= time
                    && a.detached > time
            })
            .cloned())
    }
}
