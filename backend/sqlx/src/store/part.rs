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

#[async_session::async_trait]
impl tb_domain::PartStore for SqlxConn {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        sqlx::query_as::<_, DbPart>("SELECT * FROM parts WHERE id = $1")
            .bind(i32::from(pid))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        sqlx::query_as::<_, DbPart>("SELECT * FROM parts WHERE owner = $1 ORDER BY last_used")
            .bind(i32::from(*uid))
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
        sqlx::query_as::<_, DbPart>(
            "INSERT INTO parts (owner, what, name, vendor, model, purchase, last_used, usage, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING *"
        )
        .bind(i32::from(in_owner))
        .bind(i32::from(in_what))
        .bind(in_name)
        .bind(in_vendor)
        .bind(in_model)
        .bind(in_purchase)
        .bind(in_purchase) // last_used = purchase
        .bind(Uuid::from(in_usage))
        .bind(in_source)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        let part = DbPart::from(part);
        sqlx::query_as::<_, DbPart>(
            "UPDATE parts
             SET owner = $2, what = $3, name = $4, vendor = $5, model = $6,
                 purchase = $7, last_used = $8, disposed_at = $9, usage = $10, source = $11
             WHERE id = $1
             RETURNING *",
        )
        .bind(part.id)
        .bind(part.owner)
        .bind(part.what)
        .bind(part.name)
        .bind(part.vendor)
        .bind(part.model)
        .bind(part.purchase)
        .bind(part.last_used)
        .bind(part.disposed_at)
        .bind(part.usage)
        .bind(part.source)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn part_delete(&mut self, pid: PartId) -> TbResult<PartId> {
        sqlx::query("DELETE FROM parts WHERE id = $1")
            .bind(i32::from(pid))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(pid)
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        sqlx::query_scalar::<_, i32>("SELECT id FROM parts WHERE source = $1 FOR UPDATE")
            .bind(strava_id)
            .fetch_optional(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(option_into)
    }

    async fn parts_delete(&mut self, list: &[Part]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| i32::from(s.id)).collect();

        let result = sqlx::query("DELETE FROM parts WHERE id = ANY($1)")
            .bind(&list)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
