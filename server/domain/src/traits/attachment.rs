use time::OffsetDateTime;

use crate::{AnyResult, Attachment, PartId, PartTypeId, Usage};

#[async_trait::async_trait]
pub trait AttachmentStore {
    async fn attachment_create(&mut self, att: Attachment) -> AnyResult<Attachment>;

    async fn attachment_delete(&mut self, att: Attachment) -> AnyResult<Attachment>;

    async fn attachment_reset_all(&mut self) -> AnyResult<usize>;

    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> AnyResult<Vec<Attachment>>;

    async fn attachments_add_usage_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
        usage: &Usage,
    ) -> AnyResult<Vec<Attachment>>;

    async fn attachments_all_by_partlist(&mut self, ids: Vec<PartId>)
        -> AnyResult<Vec<Attachment>>;

    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        tim: OffsetDateTime,
    ) -> AnyResult<Option<Attachment>>;

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<crate::PartType>,
        target: PartId,
        tim: OffsetDateTime,
    ) -> AnyResult<Vec<Attachment>>;

    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        what: PartTypeId,
        g: PartId,
        h: PartTypeId,
        t: OffsetDateTime,
    ) -> AnyResult<Option<Attachment>>;

    /// Return Attachment if some other part is attached to same hook after the Event
    async fn attachment_find_successor(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
        what: PartTypeId,
    ) -> AnyResult<Option<Attachment>>;

    /// Return Attachment if self.part_id is attached somewhere after the event
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id_: PartId,
        time_: OffsetDateTime,
    ) -> AnyResult<Option<Attachment>>;

    /// Iff self.part_id already attached just before self.time return that attachment
    async fn attachment_find_part_attached_already(
        &mut self,
        part_id_: PartId,
        gear_: PartId,
        hook_: PartTypeId,
        time_: OffsetDateTime,
    ) -> AnyResult<Option<Attachment>>;
}
