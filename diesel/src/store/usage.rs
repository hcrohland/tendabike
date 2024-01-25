use async_session::log::debug;
use diesel::QueryDsl;
use diesel_async::{AsyncConnection, RunQueryDsl};
use scoped_futures::ScopedFutureExt;
use std::borrow::Borrow;

use crate::{map_to_tb, AsyncDieselConn};
use tb_domain::{schema, TbResult, Usage, UsageId, UsageStore};

#[async_session::async_trait]
impl UsageStore for AsyncDieselConn {
    async fn usage_get(&mut self, id: UsageId) -> TbResult<Usage> {
        use schema::usages;
        match usages::table.find(id).get_result::<Usage>(self).await {
            Ok(x) => Ok(x),
            Err(diesel::NotFound) => Ok(Usage {
                id,
                ..Default::default()
            }),
            Err(x) => Err(map_to_tb(x)),
        }
    }

    async fn usage_update<U>(&mut self, vec: &Vec<U>) -> TbResult<usize>
    where
        U: Borrow<Usage> + Sync,
    {
        use schema::usages;

        let len = vec.len();
        self.transaction(|store| {
            async move {
                for usage in vec {
                    let usage = usage.borrow();
                    diesel::insert_into(usages::table)
                        .values(usage)
                        .on_conflict(usages::id)
                        .do_update()
                        .set(usage)
                        .execute(store)
                        .await?;
                }
                Ok(len)
            }
            .scope_boxed()
        })
        .await
    }

    async fn usage_delete(&mut self, usage: &UsageId) -> TbResult<Usage> {
        use schema::usages::dsl::*;
        diesel::delete(usages.find(usage))
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn usage_delete_all(&mut self) -> TbResult<usize> {
        use schema::usages::dsl::*;
        debug!("resetting all usages");
        diesel::delete(usages)
            .execute(self)
            .await
            .map_err(map_to_tb)
    }
}
