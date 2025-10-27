use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use uuid::Uuid;

use super::schema;
use crate::{AsyncDieselConn, into_domain, option_into, vec_into};
use tb_domain::{Part, PartId, PartTypeId, TbResult, UsageId, UserId};

/// The database's representation of a part.
#[derive(Clone, Debug, PartialEq, Queryable, Identifiable, AsChangeset)]
#[diesel(primary_key(id))]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = schema::parts)]
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
impl tb_domain::PartStore for AsyncDieselConn {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        use schema::parts;
        parts::table
            .find(i32::from(pid))
            .first::<DbPart>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        use schema::parts::dsl::*;

        parts
            .filter(owner.eq(i32::from(*uid)))
            .order_by(last_used)
            .load::<DbPart>(self)
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
        use schema::parts::dsl::*;
        let values = (
            owner.eq(i32::from(in_owner)),
            what.eq(i32::from(in_what)),
            name.eq(in_name),
            vendor.eq(in_vendor),
            model.eq(in_model),
            purchase.eq(in_purchase),
            last_used.eq(in_purchase),
            usage.eq(Uuid::from(in_usage)),
            source.eq(in_source),
        );

        diesel::insert_into(parts)
            .values(values)
            .get_result::<DbPart>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        use schema::parts::dsl::*;
        let part = DbPart::from(part);
        diesel::update(parts.find(part.id))
            .set(part)
            .get_result::<DbPart>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn part_delete(&mut self, pid: PartId) -> TbResult<PartId> {
        use schema::parts::dsl::*;
        diesel::delete(parts.find(i32::from(pid)))
            .execute(self)
            .await?;
        Ok(pid)
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        use schema::parts::dsl::*;
        parts
            .filter(source.eq(strava_id))
            .select(id)
            .for_update()
            .first::<i32>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(option_into)
    }

    async fn parts_delete(&mut self, list: &[Part]) -> TbResult<usize> {
        use schema::parts::dsl::*;

        let list: Vec<_> = list.iter().map(|s| i32::from(s.id)).collect();

        diesel::delete(parts.filter(id.eq_any(list)))
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
