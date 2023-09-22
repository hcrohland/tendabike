use crate::*;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use tb_domain::schema;
use tb_domain::TbResult;
use tb_domain::Part;
use tb_domain::PartId;
use tb_domain::Person;
use tb_domain::Usage;
use tb_domain::UserId;
use time::OffsetDateTime;

use tb_domain::PartTypeId;

#[async_session::async_trait]
impl tb_domain::PartStore for AsyncDieselConn {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        use schema::parts;
        parts::table
            .find(pid)
            .first::<Part>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn partid_get_name(&mut self, pid: PartId) -> TbResult<String> {
        use schema::parts;
        parts::table
            .find(pid)
            .select(parts::name)
            .first(self)
            .await
            .map_err(map_to_tb)
    }

    async fn partid_get_type(&mut self, pid: PartId) -> TbResult<PartTypeId> {
        use schema::parts;
        parts::table
            .find(pid)
            .select(parts::what)
            .first(self)
            .await
            .map_err(map_to_tb)
    }

    async fn partid_get_ownerid(&mut self, pid: PartId, user: &dyn Person) -> TbResult<UserId> {
        use schema::parts::dsl::*;
        parts
            .find(pid)
            .filter(owner.eq(user.get_id()))
            .select(owner)
            .first::<UserId>(self)
            .await
            .map_err(Into::into)
    }

    async fn partid_apply_usage(
        &mut self,
        pid: PartId,
        usage: &Usage,
        start: OffsetDateTime,
    ) -> TbResult<Part> {
        use schema::parts::dsl::*;
        Ok(self
            .transaction(|conn| {
                async {
                    let part: Part = parts.find(pid).for_update().get_result(conn).await?;
                    diesel::update(parts.find(pid))
                        .set((
                            time.eq(time + usage.time),
                            climb.eq(climb + usage.climb),
                            descend.eq(descend + usage.descend),
                            distance.eq(distance + usage.distance),
                            count.eq(count + usage.count),
                            purchase.eq(std::cmp::min(part.purchase, start)),
                            last_used.eq(std::cmp::max(part.last_used, start)),
                        ))
                        .get_result::<Part>(conn)
                        .await
                }
                .scope_boxed()
            })
            .await?)
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        use schema::parts::dsl::*;

        parts
            .filter(owner.eq(uid))
            .order_by(last_used)
            .load::<Part>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn parts_reset_all_usages(&mut self, uid: UserId) -> TbResult<Vec<Part>> {
        use schema::parts::dsl::*;
        diesel::update(parts.filter(owner.eq(uid)))
            .set((
                time.eq(0),
                climb.eq(0),
                descend.eq(0),
                distance.eq(0),
                count.eq(0),
                last_used.eq(purchase),
            ))
            .get_results::<Part>(self)
            .await
            .map_err(map_to_tb)
    }

    async fn create_part(
        &mut self,
        newpart: tb_domain::NewPart,
        createtime: OffsetDateTime,
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
            time.eq(0),
            distance.eq(0),
            climb.eq(0),
            descend.eq(0),
            count.eq(0),
        );

        diesel::insert_into(parts)
            .values(values)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn part_change(&mut self, part: tb_domain::ChangePart) -> TbResult<Part> {
        use schema::parts::dsl::*;
        diesel::update(parts.filter(id.eq(part.id)))
            .set(part)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }
}
