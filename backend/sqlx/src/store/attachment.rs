use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{SqlxConn, into_domain, vec_into};
use tb_domain::{Attachment, PartId, PartTypeId, TbResult};

#[derive(Clone, Copy, Debug, PartialEq, FromRow)]
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
impl<'c> tb_domain::AttachmentStore for SqlxConn<'c> {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        let att: DbAttachment = att.into();
        sqlx::query_as!(
            DbAttachment,
            "INSERT INTO attachments (part_id, attached, gear, hook, detached, usage)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
            att.part_id,
            att.attached,
            att.gear,
            att.hook,
            att.detached,
            att.usage
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        let att: DbAttachment = att.into();
        sqlx::query_as!(
            DbAttachment,
            "DELETE FROM attachments
             WHERE part_id = $1 AND attached = $2
             RETURNING *",
            att.part_id,
            att.attached
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        sqlx::query_as!(
            DbAttachment,
            "SELECT * FROM attachments
             WHERE gear = $1
               AND attached <= $2
               AND (detached IS NULL OR detached > $2)",
            i32::from(act_gear),
            start
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        sqlx::query_as!(
            DbAttachment,
            "SELECT * FROM attachments
             WHERE part_id = $1",
            i32::from(id)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        sqlx::query_as!(
            DbAttachment,
            "SELECT * FROM attachments
             WHERE part_id = $1
               AND attached <= $2
               AND detached > $2
             FOR UPDATE",
            i32::from(pid),
            time
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|a| a.map(Into::into))
    }

    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<tb_domain::PartTypeId>,
        gear_: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        let types_i32: Vec<i32> = vec_into(types);
        sqlx::query_as!(
            DbAttachment,
            "SELECT * FROM attachments
             WHERE hook = ANY($1)
               AND gear = $2
               AND attached <= $3
               AND detached > $3
             ORDER BY hook
             FOR UPDATE",
            &types_i32 as _,
            i32::from(gear_),
            time
        )
        .fetch_all(&mut **self.inner())
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
        let what_: i32 = what_.into();
        let gear_: i32 = gear_.into();
        let hook_: i32 = hook_.into();

        sqlx::query_as!(
            DbAttachment,
            "SELECT a.* FROM attachments a
             INNER JOIN parts p ON p.id = a.part_id AND p.what = $1
             WHERE a.gear = $2
               AND a.hook = $3
               AND a.attached <= $4
               AND a.detached > $4",
            what_,
            gear_,
            hook_,
            time_
        )
        .fetch_optional(&mut **self.inner())
        .await
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
        sqlx::query_as!(
            DbAttachment,
            "SELECT a.* FROM attachments a
             INNER JOIN parts p ON p.id = a.part_id AND p.what = $1
             WHERE a.gear = $2
               AND a.hook = $3
               AND a.part_id <> $4
               AND a.attached > $5
             ORDER BY a.attached
             FOR UPDATE
             LIMIT 1",
            i32::from(what),
            i32::from(gear_),
            i32::from(hook_),
            i32::from(part_id_),
            time_
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|a| a.map(Into::into))
    }

    /// Return Attachment if self.part_id is attached somewhere after the event
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id_: PartId,
        time_: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        sqlx::query_as!(
            DbAttachment,
            "SELECT * FROM attachments
             WHERE part_id = $1
               AND attached > $2
             ORDER BY attached
             FOR UPDATE
             LIMIT 1",
            i32::from(part_id_),
            time_
        )
        .fetch_optional(&mut **self.inner())
        .await
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
        sqlx::query_as!(
            DbAttachment,
            "SELECT * FROM attachments
             WHERE part_id = $1
               AND gear = $2
               AND hook = $3
               AND detached = $4
             FOR UPDATE",
            i32::from(part_id_),
            i32::from(gear_),
            i32::from(hook_),
            time_
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|a| a.map(Into::into))
    }

    async fn attachments_delete_by_parts(&mut self, list: &[tb_domain::Part]) -> TbResult<usize> {
        let list: Vec<i32> = list.iter().map(|s| i32::from(s.id)).collect();

        let result = sqlx::query!(
            "DELETE FROM attachments
             WHERE part_id = ANY($1)",
            &list as _
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
