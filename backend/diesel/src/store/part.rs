use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::{AsyncDieselConn, into_domain};
use tb_domain::{Part, PartId, PartTypeId, TbResult, UsageId, UserId, schema};

#[derive(Clone, Debug, PartialEq, Insertable)]
#[diesel(table_name = schema::parts)]
pub struct NewPart {
    /// The owner
    pub owner: UserId,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    pub purchase: Option<OffsetDateTime>,
    /// The source id tagged by the source as {"<Source>": "<Id>"}
    pub source: Option<String>,
}

#[async_session::async_trait]
impl tb_domain::PartStore for AsyncDieselConn {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        use schema::parts;
        parts::table
            .find(pid)
            .first::<Part>(self)
            .await
            .map_err(into_domain)
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        use schema::parts::dsl::*;

        parts
            .filter(owner.eq(uid))
            .order_by(last_used)
            .load::<Part>(self)
            .await
            .map_err(into_domain)
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
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn part_update(&mut self, part: &Part) -> TbResult<Part> {
        use schema::parts::dsl::*;
        diesel::update(parts.filter(id.eq(part.id)))
            .set(part)
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn part_change(
        &mut self,
        part: PartId,
        in_name: String,
        in_vendor: String,
        in_model: String,
        in_purchase: OffsetDateTime,
    ) -> TbResult<Part> {
        use schema::parts::dsl::*;
        diesel::update(parts.filter(id.eq(part)))
            .set((
                name.eq(in_name),
                vendor.eq(in_vendor),
                model.eq(in_model),
                purchase.eq(in_purchase),
            ))
            .get_result(self)
            .await
            .map_err(into_domain)
    }
}
