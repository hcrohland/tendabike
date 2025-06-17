use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{AsyncDieselConn, into_domain, vec_into};
use tb_domain::{Attachment, PartId, PartTypeId, TbResult, schema};

use schema::attachments::table;

#[derive(Clone, Copy, Debug, PartialEq, Queryable, Identifiable, Insertable, AsChangeset)]
#[diesel(primary_key(part_id, attached))]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = schema::attachments)]
pub struct DbAttachment {
    part_id: i32,
    attached: OffsetDateTime,
    gear: i32,
    hook: i32,
    detached: OffsetDateTime,
    usage: Uuid,
}

impl From<Attachment> for DbAttachment {
    fn from(value: Attachment) -> Self {
        let Attachment {
            part_id,
            attached,
            gear,
            hook,
            detached,
            usage,
        } = value;
        Self {
            part_id: part_id.into(),
            attached,
            gear: gear.into(),
            hook: hook.into(),
            detached,
            usage: usage.into(),
        }
    }
}
impl From<DbAttachment> for Attachment {
    fn from(value: DbAttachment) -> Self {
        let DbAttachment {
            part_id,
            attached,
            gear,
            hook,
            detached,
            usage,
        } = value;
        Self {
            part_id: part_id.into(),
            attached,
            gear: gear.into(),
            hook: hook.into(),
            detached,
            usage: usage.into(),
        }
    }
}

#[async_session::async_trait]
impl tb_domain::AttachmentStore for AsyncDieselConn {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        let att: DbAttachment = att.into();
        att.insert_into(table)
            .get_result::<DbAttachment>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        let att: DbAttachment = att.into();
        diesel::delete(table.find(att.id())) // delete the attachment in the database
            .get_result::<DbAttachment>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
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
            .get_results::<DbAttachment>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        attachments
            .filter(part_id.eq(id))
            .get_results::<DbAttachment>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        let pid: i32 = pid.into();
        table
            .for_update()
            .filter(part_id.eq(pid))
            .filter(attached.le(time))
            .filter(detached.gt(time))
            .first::<DbAttachment>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(|a| a.map(Into::into))
    }

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<tb_domain::PartTypeId>,
        gear_: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;
        let gear_: i32 = gear_.into();
        let types: Vec<i32> = vec_into(types);
        attachments
            .for_update()
            .filter(hook.eq_any(types))
            .filter(gear.eq(gear_))
            .filter(attached.le(time))
            .filter(detached.gt(time))
            .order(hook)
            .load::<DbAttachment>(self)
            .await
            .map_err(into_domain)
            .map(vec_into)
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
        let what_: i32 = what_.into();
        let gear_: i32 = gear_.into();
        let hook_: i32 = hook_.into();

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
            .first::<DbAttachment>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(|a| a.map(Into::into))
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
        let part_id_: i32 = part_id_.into();
        let gear_: i32 = gear_.into();
        let hook_: i32 = hook_.into();
        let what: i32 = what.into();

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
            .first::<DbAttachment>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(|a| a.map(Into::into))
    }

    /// Return Attachment if self.part_id is attached somewhere after the event
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id_: PartId,
        time_: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        let part_id_: i32 = part_id_.into();
        attachments
            .for_update()
            .filter(part_id.eq(part_id_))
            .filter(attached.gt(time_))
            .order(attached)
            .first::<DbAttachment>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(|a| a.map(Into::into))
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
        let part_id_: i32 = part_id_.into();
        let gear_: i32 = gear_.into();
        let hook_: i32 = hook_.into();

        attachments
            .for_update()
            .filter(part_id.eq(part_id_))
            .filter(gear.eq(gear_))
            .filter(hook.eq(hook_))
            .filter(detached.eq(time_))
            .first::<DbAttachment>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(|a| a.map(Into::into))
    }
}
