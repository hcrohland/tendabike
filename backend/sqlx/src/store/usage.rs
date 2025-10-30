use async_session::log::debug;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use scoped_futures::ScopedFutureExt;
use std::borrow::Borrow;
use uuid::Uuid;

use super::schema;
use crate::{AsyncDieselConn, into_domain, option_into};
use tb_domain::{TbResult, Usage, UsageId, UsageStore};

#[derive(Clone, Debug, PartialEq, Default, Queryable, Identifiable, AsChangeset, Insertable)]
#[diesel(table_name = schema::usages)]
pub struct DbUsage {
    // id for referencing
    pub id: Uuid,
    // usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
    /// Overall energy
    pub energy: i32,
    /// number of activities
    pub count: i32,
}

impl From<&Usage> for DbUsage {
    fn from(value: &Usage) -> Self {
        let &Usage {
            id,
            time,
            distance,
            climb,
            descend,
            energy,
            count,
        } = value;
        Self {
            id: id.into(),
            time,
            distance,
            climb,
            descend,
            energy,
            count,
        }
    }
}
impl From<DbUsage> for Usage {
    fn from(value: DbUsage) -> Self {
        let DbUsage {
            id,
            time,
            distance,
            climb,
            descend,
            energy,
            count,
        } = value;
        Self {
            id: id.into(),
            time,
            distance,
            climb,
            descend,
            energy,
            count,
        }
    }
}

#[async_session::async_trait]
impl UsageStore for AsyncDieselConn {
    async fn get(&mut self, id: UsageId) -> TbResult<Option<Usage>> {
        use diesel::result::OptionalExtension;
        use schema::usages;
        usages::table
            .find(Uuid::from(id))
            .get_result::<DbUsage>(self)
            .await
            .optional()
            .map_err(into_domain)
            .map(option_into)
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
                    let usage = DbUsage::from(usage.borrow());
                    diesel::insert_into(usages::table)
                        .values(&usage)
                        .on_conflict(usages::id)
                        .do_update()
                        .set(&usage)
                        .execute(store)
                        .await?;
                }
                Ok(len)
            }
            .scope_boxed()
        })
        .await
    }

    async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        use schema::usages::dsl::*;
        diesel::delete(usages.find(Uuid::from(usage)))
            .get_result::<DbUsage>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        use schema::usages::dsl::*;
        debug!("resetting all usages");
        diesel::delete(usages)
            .execute(self)
            .await
            .map_err(into_domain)
    }

    async fn usages_delete(&mut self, list: &[Usage]) -> TbResult<usize> {
        use schema::usages::dsl::*;

        let list: Vec<_> = list.iter().map(|s| Uuid::from(s.id)).collect();

        diesel::delete(usages.filter(id.eq_any(list)))
            .execute(self)
            .await
            .map_err(into_domain)
    }
}
