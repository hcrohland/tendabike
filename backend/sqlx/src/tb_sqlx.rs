//! This module contains the implementation of the database connection pool.
//!
//! The `DbPool` struct is a wrapper around the `sqlx::PgPool` type, which is used to manage a pool of database connections.
//! The `SqlxConn` struct is a wrapper around the `sqlx::pool::PoolConnection<sqlx::Postgres>` type, which represents a single database connection.
//!
//! This module is used by other modules in the application to interact with the database.

use anyhow::Context;
use log::info;
use sqlx::{PgPool, PgTransaction};
use std::ops::{Deref, DerefMut};

use tb_domain::TbResult;

pub struct SqlxConn<'conn>(PgTransaction<'conn>);

impl<'c> Deref for SqlxConn<'c> {
    type Target = PgTransaction<'c>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'c> DerefMut for SqlxConn<'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'c> SqlxConn<'c> {
    pub(crate) fn inner(&mut self) -> &mut PgTransaction<'c> {
        &mut self.0
    }
}

impl<'c> SqlxConn<'c> {
    pub(crate) fn into_inner(self) -> PgTransaction<'c> {
        self.0
    }
}

#[derive(Clone)]
pub struct DbPool(PgPool);

impl DbPool {
    pub async fn new() -> anyhow::Result<Self> {
        let database_url =
            std::env::var("DB_URL").unwrap_or("postgres://localhost/tendabike".to_string());

        info!("Connecting to database: {}", database_url);

        let pool = PgPool::connect(&database_url).await?;

        // Run migrations if needed
        sqlx::migrate!("./migrations").run(&pool).await?;

        let pool = DbPool(pool);

        Ok(pool)
    }

    pub async fn begin(&self) -> TbResult<SqlxConn<'_>> {
        let conn = self
            .0
            .begin()
            .await
            .context("Could not get pool connection")?;
        Ok(SqlxConn(conn))
    }
}
