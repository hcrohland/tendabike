use anyhow::Context;
use async_session::log::debug;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, Insertable, Identifiable};
use diesel_async::RunQueryDsl;
use s_diesel::{schema, AppConn};
use time::OffsetDateTime;

use crate::{AnyResult, Attachment, PartId, Usage};

#[async_session::async_trait]
impl crate::traits::AttachmentStore for AppConn {
    async fn attachment_create(&mut self, att: Attachment) -> AnyResult<Attachment> {
        att
            .insert_into(schema::attachments::table)
            .get_result::<Attachment>(self)
            .await
            .context("insert into attachments")
    }

    async fn attachment_delete(&mut self, att: Attachment) -> AnyResult<Attachment> {
        diesel::delete(schema::attachments::table.find(att.id())) // delete the attachment in the database
            .get_result::<Attachment>(self)
            .await
            .context(format!("Could not delete attachment {:#?}", att))
    }

    
    async fn attachment_reset_all(&mut self) -> AnyResult<usize> {
        use schema::attachments::dsl::*;
        debug!("resetting all attachments");
        diesel::update(attachments)
            .set((descend.eq(0), count.eq(0)))
            .execute(self)
            .await
            .context("Could not reset attachments")
    }

    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> AnyResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .filter(gear.eq(act_gear))
            .filter(attached.lt(start))
            .filter(detached.is_null().or(detached.ge(start)))
            .get_results::<Attachment>(self)
            .await
            .context("Error reading attachments")
    }

    async fn attachments_add_usage_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
        usage: &Usage,
    ) -> AnyResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        diesel::update(
            attachments
                .filter(gear.eq(act_gear))
                .filter(attached.lt(start))
                .filter(detached.ge(start)),
        )
        .set((
            time.eq(time + usage.time),
            climb.eq(climb + usage.climb),
            descend.eq(descend + usage.descend),
            distance.eq(distance + usage.distance),
            count.eq(count + usage.count),
        ))
        .get_results::<Attachment>(self)
        .await
        .context("update attachments failed")
    }

    async fn attachments_all_by_partlist(
        &mut self,
        ids: Vec<PartId>,
    ) -> AnyResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .filter(part_id.eq_any(ids.clone()))
            .or_filter(gear.eq_any(ids))
            .get_results(self)
            .await
            .context("get attachments")
    }
}
