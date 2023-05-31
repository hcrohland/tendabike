//! This module contains the implementation of the database connection pool and the database migration logic.
//!
//! The `DbPool` struct is a wrapper around the `r2d2::Pool` type, which is used to manage a pool of database connections.
//! The `Store` struct is a wrapper around the `diesel::PgConnection` type, which is used to represent a single database connection.
//! The `run_db_migrations` function is used to run the database migrations using the `diesel_migrations` crate.
//!
//! This module is used by other modules in the application to interact with the database.

use crate::domain::AnyResult;

use async_session::log::info;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::{Pool, Object};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;

pub type AppConn = Object<AsyncPgConnection>;

use diesel_migrations::MigrationHarness;
pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    embed_migrations!("src/s_diesel/migrations");

fn run_db_migrations(db: &str) {
    info!("Running database migrations...");
    let mut conn = PgConnection::establish(&db).expect("Failed to connect to database: {:?}");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run database migrations: {:?}");
}
#[derive(Clone)]
pub struct DbPool (Pool<AsyncPgConnection>);

impl DbPool {
    pub async fn new() -> AnyResult<Self> {
        let database_url =
            std::env::var("DB_URL").unwrap_or("postgres://localhost/tendabike".to_string());
        run_db_migrations(&database_url);

        let config =
            AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);
        let pool: Pool<AsyncPgConnection> = Pool::builder(config).build()?;

        Ok(DbPool(pool))
    }

    pub async fn get(&self) -> AnyResult<diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>> {
        let conn = self.0.get().await?;
        Ok(conn)
    }
}
