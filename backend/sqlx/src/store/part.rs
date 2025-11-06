use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{SqlxConn, into_domain, option_into, vec_into};
use tb_domain::{Part, PartId, PartTypeId, TbResult, UsageId, UserId};

/// The database's representation of a part.
#[derive(Clone, Debug, PartialEq, FromRow)]
struct DbPart {
    id: i32,
    owner: i32,
    what: i32,
    name: String,
    vendor: String,
    model: String,
    purchase: OffsetDateTime,
    last_used: OffsetDateTime,
    disposed_at: Option<OffsetDateTime>,
    usage: uuid::Uuid,
    source: Option<String>,
}

impl From<DbPart> for Part {
    fn from(db: DbPart) -> Self {
        let DbPart {
            id,
            owner,
            what,
            name,
            vendor,
            model,
            purchase,
            last_used,
            disposed_at,
            usage,
            source,
        } = db;
        Self {
            id: id.into(),
            owner: owner.into(),
            what: what.into(),
            name,
            vendor,
            model,
            purchase,
            last_used,
            disposed_at,
            usage: usage.into(),
            source,
        }
    }
}

impl From<Part> for DbPart {
    fn from(value: Part) -> Self {
        let Part {
            id,
            owner,
            what,
            name,
            vendor,
            model,
            purchase,
            last_used,
            disposed_at,
            usage,
            source,
        } = value;
        Self {
            id: id.into(),
            owner: owner.into(),
            what: what.into(),
            name,
            vendor,
            model,
            purchase,
            last_used,
            disposed_at,
            usage: usage.into(),
            source,
        }
    }
}

#[async_trait::async_trait]
impl<'c> tb_domain::PartStore for SqlxConn<'c> {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        sqlx::query_as!(DbPart, "SELECT * FROM parts WHERE id = $1", i32::from(pid))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        sqlx::query_as!(
            DbPart,
            "SELECT * FROM parts WHERE owner = $1 ORDER BY last_used",
            i32::from(*uid)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn part_create(
        &mut self,
        in_what: PartTypeId,
        in_name: String,
        in_vendor: String,
        in_model: String,
        in_purchase: OffsetDateTime,
        in_source: Option<String>,
        in_usage: UsageId,
        in_owner: UserId,
    ) -> TbResult<Part> {
        sqlx::query_as!(
            DbPart,
            "INSERT INTO parts (owner, what, name, vendor, model, purchase, last_used, usage, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING *",
            i32::from(in_owner),
            i32::from(in_what),
            in_name,
            in_vendor,
            in_model,
            in_purchase,
            in_purchase, // last_used = purchase
            Uuid::from(in_usage),
            in_source
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        let part = DbPart::from(part);
        sqlx::query_as!(
            DbPart,
            "UPDATE parts
             SET owner = $2, what = $3, name = $4, vendor = $5, model = $6,
                 purchase = $7, last_used = $8, disposed_at = $9, usage = $10, source = $11
             WHERE id = $1
             RETURNING *",
            part.id,
            part.owner,
            part.what,
            part.name,
            part.vendor,
            part.model,
            part.purchase,
            part.last_used,
            part.disposed_at,
            part.usage,
            part.source
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn part_delete(&mut self, pid: PartId) -> TbResult<PartId> {
        sqlx::query!("DELETE FROM parts WHERE id = $1", i32::from(pid))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(pid)
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        sqlx::query_scalar!(
            "SELECT id FROM parts WHERE source = $1 FOR UPDATE",
            strava_id
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn parts_delete(&mut self, list: &[Part]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| i32::from(s.id)).collect();

        let result = sqlx::query!("DELETE FROM parts WHERE id = ANY($1)", &list as _)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
