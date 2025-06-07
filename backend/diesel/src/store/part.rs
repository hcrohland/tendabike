use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::{AsyncDieselConn, into_domain};
use tb_domain::{Part, PartId, TbResult, UsageId, UserId, schema};

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
        newpart: tb_domain::NewPart,
        createtime: OffsetDateTime,
        usage_: UsageId,
    ) -> TbResult<Part> {
        use schema::parts::dsl::*;
        let values = (
            owner.eq(newpart.owner),
            what.eq(newpart.what),
            name.eq(newpart.name),
            vendor.eq(newpart.vendor),
            model.eq(newpart.model),
            purchase.eq(createtime),
            last_used.eq(createtime),
            usage.eq(usage_),
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

    async fn part_change(&mut self, part: tb_domain::ChangePart) -> TbResult<Part> {
        use schema::parts::dsl::*;
        diesel::update(parts.filter(id.eq(part.id)))
            .set(part)
            .get_result(self)
            .await
            .map_err(into_domain)
    }
}
