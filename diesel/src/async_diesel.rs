//! This module contains the implementation of the database connection pool and the database migration logic.
//!
//! The `DbPool` struct is a wrapper around the `r2d2::Pool` type, which is used to manage a pool of database connections.
//! The `Store` struct is a wrapper around the `diesel::PgConnection` type, which is used to represent a single database connection.
//! The `run_db_migrations` function is used to run the database migrations using the `diesel_migrations` crate.
//!
//! This module is used by other modules in the application to interact with the database.

use std::ops::{Deref, DerefMut};

use async_session::log::info;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use anyhow::Result as AnyResult;

type MyConnection = AsyncPgConnection;
pub struct AsyncDieselConn(Object<MyConnection>);

impl Deref for AsyncDieselConn {
    type Target = Object<MyConnection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AsyncDieselConn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

use diesel_migrations::{embed_migrations, MigrationHarness};
pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    embed_migrations!("migrations");

fn run_db_migrations(db: &str) {
    info!("Running database migrations...");
    let mut conn = PgConnection::establish(&db).expect("Failed to connect to database: {:?}");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run database migrations: {:?}");
}
#[derive(Clone)]
pub struct DbPool(Pool<MyConnection>);

impl DbPool {
    pub async fn new() -> AnyResult<Self> {
        let database_url =
            std::env::var("DB_URL").unwrap_or("postgres://localhost/tendabike".to_string());
        run_db_migrations(&database_url);

        let config =
            AsyncDieselConnectionManager::<MyConnection>::new(database_url);
        let pool: Pool<MyConnection> = Pool::builder(config).build()?;

        Ok(DbPool(pool))
    }

    pub async fn get(
        &self,
    ) -> AnyResult<AsyncDieselConn> {
        let conn = self.0.get().await?;
        Ok(AsyncDieselConn(conn))
    }
}
