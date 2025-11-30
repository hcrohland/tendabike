use log::debug;
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
    pub time: Option<i32>,
    /// Usage distance
    pub distance: Option<i32>,
    /// Overall climbing
    pub climb: Option<i32>,
    /// Overall descending
    pub descend: Option<i32>,
    /// Overall energy
    pub energy: i32,
    /// number of activities
    pub count: Option<i32>,
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
            time: Some(time),
            distance: Some(distance),
            climb: Some(climb),
            descend: Some(descend),
            energy,
            count: Some(count),
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
            time: time.unwrap_or(0),
            distance: distance.unwrap_or(0),
            climb: climb.unwrap_or(0),
            descend: descend.unwrap_or(0),
            energy,
            count: count.unwrap_or(0),
        }
    }
}

#[async_trait::async_trait]
impl<'c> UsageStore for SqlxConn<'c> {
    async fn get(&mut self, id: UsageId) -> TbResult<Option<Usage>> {
        sqlx::query_as!(
            DbUsage,
            r#"SELECT id, time, distance, climb, descend, energy as "energy!", count FROM usages WHERE id = $1"#,
            Uuid::from(id)
        )
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
            sqlx::query!(
                "INSERT INTO usages (id, time, distance, climb, descend, energy, count)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE
                 SET time = $2, distance = $3, climb = $4, descend = $5, energy = $6, count = $7",
                usage.id,
                usage.time,
                usage.distance,
                usage.climb,
                usage.descend,
                usage.energy,
                usage.count
            )
            .execute(&mut *tx)
            .await
            .map_err(into_domain)?;
        }

        // Commit the transaction
        tx.commit().await.map_err(into_domain)?;

        Ok(len)
    }

    async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        sqlx::query_as!(
            DbUsage,
            r#"DELETE FROM usages WHERE id = $1 RETURNING id, time, distance, climb, descend, energy as "energy!", count"#,
            Uuid::from(usage)
        )
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        debug!("resetting all usages");
        let result = sqlx::query!("DELETE FROM usages")
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn usages_delete(&mut self, list: &[Usage]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| Uuid::from(s.id)).collect();

        let result = sqlx::query!("DELETE FROM usages WHERE id = ANY($1)", &list as _)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
