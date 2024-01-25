use time::OffsetDateTime;

use crate::{Attachment, PartId, PartTypeId, TbResult};

/// This trait defines methods for storing and retrieving attachments.
#[async_trait::async_trait]
pub trait AttachmentStore {
    /// Create a new attachment.
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment>;

    /// Delete an attachment.
    async fn attachment_delete(&mut self, att: Attachment) -> TbResult<Attachment>;

    /// Get all attachments for a given gear and time.
    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>>;

    /// Get all attachments for a list of part IDs.
    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>>;

    /// Get an attachment for a given part and time.
    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        tim: OffsetDateTime,
    ) -> TbResult<Option<Attachment>>;

    /// Get all attachments for a given set of part types, gear, and time.
    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<crate::PartType>,
        target: PartId,
        tim: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>>;

    /// Find an attachment for a given part type, gear, hook, and time.
    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        what: PartTypeId,
        g: PartId,
        h: PartTypeId,
        t: OffsetDateTime,
    ) -> TbResult<Option<Attachment>>;

    /// Find the attachment that succeeds a given part.
    async fn attachment_find_successor(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
        what: PartTypeId,
    ) -> TbResult<Option<Attachment>>;

    /// Find the attachment that is attached to a given part after a given time.
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>>;

    /// Find the attachment that is already attached to a given part just before a given time.
    async fn attachment_find_part_attached_already(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>>;
}
