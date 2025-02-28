use async_session::log::debug;
use diesel::QueryDsl;
use diesel_async::{AsyncConnection, RunQueryDsl};
use scoped_futures::ScopedFutureExt;
use std::borrow::Borrow;

use crate::{AsyncDieselConn, into_domain};
use tb_domain::{TbResult, Usage, UsageId, UsageStore, schema};

#[async_session::async_trait]
impl UsageStore for AsyncDieselConn {
    async fn get(&mut self, id: UsageId) -> TbResult<Option<Usage>> {
        use diesel::result::OptionalExtension;
        use schema::usages;
        usages::table
            .find(id)
            .get_result::<Usage>(self)
            .await
            .optional()
            .map_err(into_domain)
    }

    async fn update<U>(&mut self, vec: &[U]) -> TbResult<usize>
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

    async fn delete(&mut self, usage: &UsageId) -> TbResult<Usage> {
        use schema::usages::dsl::*;
        diesel::delete(usages.find(usage))
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        use schema::usages::dsl::*;
        debug!("resetting all usages");
        diesel::delete(usages)
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
