use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use uuid::Uuid;

use super::schema;
use crate::{AsyncDieselConn, into_domain, vec_into};
use tb_domain::{Part, PartId, PartTypeId, TbResult, UsageId, UserId};

/// The database's representation of a part.
#[derive(Clone, Debug, PartialEq, Queryable, Identifiable, AsChangeset)]
#[diesel(primary_key(id))]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = schema::parts)]
struct DbPart {
    /// The primary key
    pub id: i32,
    /// The owner
    pub owner: i32,
    /// The type of the part
    pub what: i32,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    /// purchase date
    pub purchase: OffsetDateTime,
    /// last time it was used
    pub last_used: OffsetDateTime,
    /// Was it disposed? If yes, when?
    pub disposed_at: Option<OffsetDateTime>,
    /// the usage tracker
    pub usage: uuid::Uuid,
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
        } = db;
        Part {
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
        let values = (
            owner.eq(i32::from(part.owner)),
            // what.eq(part.what),
            name.eq(part.name),
            vendor.eq(part.vendor),
            model.eq(part.model),
            purchase.eq(part.purchase),
            last_used.eq(part.purchase),
            disposed_at.eq(part.disposed_at),
            // usage.eq(part.usage),
            // source.eq(part.source),
        );
        diesel::update(parts.find(i32::from(part.id)))
            .set(values)
            .get_result::<DbPart>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }
}
