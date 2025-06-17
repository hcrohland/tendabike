use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::{AsyncDieselConn, into_domain, vec_into};
use tb_domain::{Part, PartId, PartTypeId, TbResult, UsageId, UserId, schema};

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
            source,
        }
    }
}

#[async_session::async_trait]
impl tb_domain::PartStore for AsyncDieselConn {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        use schema::parts;
        parts::table
            .find(pid)
            .first::<DbPart>(self)
            .await
            .map(Into::into)
            .map_err(into_domain)
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        use schema::parts::dsl::*;

        vec_into(
            parts
                .filter(owner.eq(uid))
                .order_by(last_used)
                .load::<DbPart>(self)
                .await,
        )
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
            owner.eq(in_owner),
            what.eq(in_what),
            name.eq(in_name),
            vendor.eq(in_vendor),
            model.eq(in_model),
            purchase.eq(in_purchase),
            last_used.eq(in_purchase),
            usage.eq(in_usage),
            source.eq(in_source),
        );

        diesel::insert_into(parts)
            .values(values)
            .get_result::<DbPart>(self)
            .await
            .map(Into::into)
            .map_err(into_domain)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        use schema::parts::dsl::*;
        let values = (
            owner.eq(part.owner),
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
        diesel::update(parts.filter(id.eq(part.id)))
            .set(values)
            .get_result::<DbPart>(self)
            .await
            .map(Into::into)
            .map_err(into_domain)
    }
}
