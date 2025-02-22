use diesel::prelude::*;
use diesel::{BoolExpressionMethods, ExpressionMethods, Identifiable, Insertable, QueryDsl};
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::{AsyncDieselConn, into_domain};
use tb_domain::{Attachment, PartId, PartTypeId, TbResult, schema};

#[async_session::async_trait]
impl tb_domain::AttachmentStore for AsyncDieselConn {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        att.insert_into(schema::attachments::table)
            .get_result::<Attachment>(self)
            .await
            .map_err(into_domain)
    }

    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        diesel::delete(schema::attachments::table.find(att.id())) // delete the attachment in the database
            .get_result::<Attachment>(self)
            .await
            .map_err(into_domain)
    }

    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .filter(gear.eq(act_gear))
            .filter(attached.le(start))
            .filter(detached.is_null().or(detached.gt(start)))
            .get_results::<Attachment>(self)
            .await
            .map_err(into_domain)
    }

    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .filter(part_id.eq(id))
            .get_results(self)
            .await
            .map_err(into_domain)
    }

    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .for_update()
            .filter(part_id.eq(pid))
            .filter(attached.le(time))
            .filter(detached.gt(time))
            .first::<Attachment>(self)
            .await
            .optional()
            .map_err(into_domain)
    }

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<tb_domain::PartTypeId>,
        gear_: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .for_update()
            .filter(hook.eq_any(types))
            .filter(gear.eq(gear_))
            .filter(attached.le(time))
            .filter(detached.gt(time))
            .order(hook)
            .load(self)
            .await
            .map_err(into_domain)
    }

    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        what_: PartTypeId,
        gear_: PartId,
        hook_: PartTypeId,
        time_: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        use schema::parts;
        attachments
            .inner_join(
                parts::table.on(parts::id
                    .eq(part_id) // join corresponding part
                    .and(parts::what.eq(what_))),
            ) // where the part has my type
            .filter(gear.eq(gear_))
            .filter(hook.eq(hook_))
            .filter(attached.le(time_))
            .filter(detached.gt(time_))
            .select(schema::attachments::all_columns) // return only the attachment
            .first::<Attachment>(self)
            .await
            .optional()
            .map_err(into_domain)
    }

    /// Return Attachment if some other part is attached to same hook after the Event
    async fn attachment_find_successor(
        &mut self,
        part_id_: PartId,
        gear_: PartId,
        hook_: PartTypeId,
        time_: OffsetDateTime,
        what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        use schema::parts;

        attachments
            .for_update()
            .inner_join(
                parts::table.on(parts::id
                    .eq(part_id) // join corresponding part
                    .and(parts::what.eq(what))),
            ) // where the part has my type
            .filter(gear.eq(gear_))
            .filter(hook.eq(hook_))
            .filter(part_id.ne(part_id_))
            .select(schema::attachments::all_columns) // return only the attachment
            .filter(attached.gt(time_))
            .order(attached)
            .first::<Attachment>(self)
            .await
            .optional()
            .map_err(into_domain)
    }

    /// Return Attachment if self.part_id is attached somewhere after the event
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id_: PartId,
        time_: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .for_update()
            .filter(part_id.eq(part_id_))
            .filter(attached.gt(time_))
            .order(attached)
            .first::<Attachment>(self)
            .await
            .optional()
            .map_err(into_domain)
    }

    /// Iff self.part_id already attached just before self.time return that attachment
    async fn attachment_find_part_attached_already(
        &mut self,
        part_id_: PartId,
        gear_: PartId,
        hook_: PartTypeId,
        time_: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .for_update()
            .filter(part_id.eq(part_id_))
            .filter(gear.eq(gear_))
            .filter(hook.eq(hook_))
            .filter(detached.eq(time_))
            .first::<Attachment>(self)
            .await
            .optional()
            .map_err(into_domain)
    }
}
