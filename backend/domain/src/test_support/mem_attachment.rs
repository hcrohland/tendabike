use super::*;
use crate::{Attachment, Error, PartId, PartTypeId, TbResult};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl AttachmentStore for MemStore {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        self.check_fault(Fault::AttachmentCreate)?;
        let d = self.state_mut();
        // The database keys attachments by (part_id, attached_time) and
        // rejects a duplicate key; mirror that here — including the error
        // it surfaces as (issue #409).
        let key = (att.part_id, att.attached);
        if d.attachments.contains_key(&key) {
            return Err(Error::DatabaseFailure(anyhow::anyhow!(
                "duplicate attachment: part {} at {:?}",
                att.part_id,
                att.attached
            )));
        }
        d.attachments.insert(key, att);
        Ok(att)
    }

    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        let d = self.state_mut();
        // The row is addressed by part + attach time — the database's key
        // (one rule on both stores); the other fields are not part of the
        // identity.
        match d.attachments.remove(&(att.part_id, att.attached)) {
            Some(a) => Ok(a),
            None => Err(crate::Error::NotFound(format!(
                "Attachment {} at {:?} not found",
                att.part_id, att.attached
            ))),
        }
    }

    async fn attachments_delete_by_parts(&mut self, parts: &[crate::Part]) -> TbResult<usize> {
        let d = self.state_mut();
        let part_ids: Vec<PartId> = parts.iter().map(|p| p.id).collect();
        let before_count = d.attachments.len();
        d.attachments.retain(|(pid, _), _| !part_ids.contains(pid));
        Ok(before_count - d.attachments.len())
    }

    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        let d = self.state();
        Ok(d.attachments
            .values()
            .filter(|a| a.gear == act_gear && a.attached <= start && a.detached > start)
            .cloned()
            .collect())
    }

    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        let d = self.state();
        let mut result: Vec<Attachment> = d
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
        let d = self.state();
        Ok(d.attachments
            .values()
            .find(|a| a.part_id == pid && a.attached <= time && a.detached > time)
            .cloned())
    }

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<PartTypeId>,
        gear: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        let d = self.state();
        Ok(d.attachments
            .values()
            .filter(|a| {
                types.contains(&a.hook) && a.gear == gear && a.attached <= time && a.detached > time
            })
            .cloned()
            .collect())
    }

    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        what: PartTypeId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        let d = self.state();
        Ok(d.attachments
            .values()
            .find(|a| {
                a.gear == gear
                    && a.hook == hook
                    && a.attached <= time
                    && a.detached > time
                    && d.parts.get(&a.part_id).is_some_and(|p| p.what == what)
            })
            .cloned())
    }

    async fn attachment_find_successor(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
        what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        let d = self.state();
        // The successor: another part, of the same type, at the same hook,
        // attached later — the part's own later rows never count (one rule
        // on both stores).
        let successor = d
            .attachments
            .values()
            .filter(|a| {
                a.part_id != part_id
                    && a.gear == gear
                    && a.hook == hook
                    && a.attached > time
                    && d.parts.get(&a.part_id).is_some_and(|p| p.what == what)
            })
            .min_by_key(|a| a.attached);

        Ok(successor.cloned())
    }

    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        let d = self.state();
        Ok(d.attachments
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
        let d = self.state();
        // The adjacent-merge trigger: the row of this part at this gear and
        // hook that ended exactly at `time` (one rule on both stores).
        Ok(d.attachments
            .values()
            .find(|a| {
                a.part_id == part_id && a.gear == gear && a.hook == hook && a.detached == time
            })
            .cloned())
    }
}
