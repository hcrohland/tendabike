use time::OffsetDateTime;

use crate::{AnyResult, Attachment, PartId, Usage};

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
}
