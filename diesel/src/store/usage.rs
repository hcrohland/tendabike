use crate::*;
use async_session::log::debug;
use diesel::QueryDsl;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use scoped_futures::ScopedFutureExt;
use tb_domain::{TbResult, Usage, UsageId, UsageStore};

#[async_session::async_trait]
impl UsageStore for AsyncDieselConn {
    async fn usage_get(&mut self, id: UsageId) -> TbResult<Usage> {
        use schema::usages;
        match usages::table
            .find(usages::id)
            .get_result::<Usage>(self)
            .await
        {
            Ok(x) => Ok(x),
            Err(diesel::NotFound) => Ok(Usage {
                id,
                ..Default::default()
            }),
            Err(x) => Err(map_to_tb(x)),
        }
    }

    async fn usage_update(&mut self, vec: Vec<&Usage>) -> TbResult<usize> {
        use schema::usages;

        let len = vec.len();
        self.transaction(|conn| {
            async move {
                for usage in vec {
                    diesel::insert_into(usages::table)
                        .values(usage)
                        .on_conflict(usages::id)
                        .do_update()
                        .set(usage)
                        .execute(conn)
                        .await?;
                }
                Ok(len)
            }
            .scope_boxed()
        })
        .await
    }

    async fn usage_reset_all(&mut self) -> TbResult<usize> {
        use schema::usages::dsl::*;
        debug!("resetting all usages");
        diesel::delete(usages)
            .execute(self)
            .await
            .map_err(map_to_tb)
    }
}
