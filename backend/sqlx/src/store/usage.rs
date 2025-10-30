use async_session::log::debug;
use sqlx::{Acquire, FromRow};
use std::borrow::Borrow;
use uuid::Uuid;

use crate::{SqlxConn, into_domain, option_into};
use tb_domain::{TbResult, Usage, UsageId, UsageStore};

#[derive(Clone, Debug, PartialEq, Default, FromRow)]
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
impl UsageStore for SqlxConn {
    async fn get(&mut self, id: UsageId) -> TbResult<Option<Usage>> {
        sqlx::query_as::<_, DbUsage>("SELECT * FROM usages WHERE id = $1")
            .bind(Uuid::from(id))
            .fetch_optional(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(option_into)
    }

    async fn update<U>(&mut self, vec: &[U]) -> TbResult<usize>
    where
        U: Borrow<Usage> + Sync,
    {
        let len = vec.len();

        // Start a SQLx transaction
        let mut tx = self.begin().await.map_err(into_domain)?;

        for usage in vec {
            let usage = DbUsage::from(usage.borrow());
            sqlx::query(
                "INSERT INTO usages (id, time, distance, climb, descend, energy, count)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE
                 SET time = $2, distance = $3, climb = $4, descend = $5, energy = $6, count = $7",
            )
            .bind(usage.id)
            .bind(usage.time)
            .bind(usage.distance)
            .bind(usage.climb)
            .bind(usage.descend)
            .bind(usage.energy)
            .bind(usage.count)
            .execute(&mut *tx)
            .await
            .map_err(into_domain)?;
        }

        // Commit the transaction
        tx.commit().await.map_err(into_domain)?;

        Ok(len)
    }

    async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        sqlx::query_as::<_, DbUsage>("DELETE FROM usages WHERE id = $1 RETURNING *")
            .bind(Uuid::from(usage))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        debug!("resetting all usages");
        let result = sqlx::query("DELETE FROM usages")
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn usages_delete(&mut self, list: &[Usage]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| Uuid::from(s.id)).collect();

        let result = sqlx::query("DELETE FROM usages WHERE id = ANY($1)")
            .bind(&list)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
