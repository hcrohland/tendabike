use std::ops::{Deref, DerefMut};

use crate::domain::AnyResult;

use anyhow::Context;
use async_session::log::info;
use diesel::prelude::*;

pub type AppConn = PgConnection;
pub struct Store(PooledConnection<ConnectionManager<AppConn>>);

impl Deref for Store {
    type Target = AppConn;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Store {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Store {
    pub fn new(pool: &DbPool) -> AnyResult<Self> {
        let conn = pool.0.get()?;
        Ok(Store(conn))
    }
}

use diesel_migrations::MigrationHarness;
pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    embed_migrations!("src/s_diesel/migrations");

fn run_db_migrations(conn: &mut AppConn) {
    info!("Running database migrations...");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run database migrations: {:?}");
}

use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};

#[derive(Clone)]
pub struct DbPool(pub r2d2::Pool<ConnectionManager<AppConn>>);

impl DbPool {
    pub fn init() -> AnyResult<r2d2::Pool<ConnectionManager<AppConn>>> {
        let database_url =
            std::env::var("DB_URL").unwrap_or("postgres://localhost/tendabike".to_string());

        println!("Connecting to database {}...", database_url);
        let manager = ConnectionManager::<AppConn>::new(database_url);

        let pool = Pool::builder()
            .build(manager)
            .context("Failed to create database connection pool.")?;

        let mut conn = pool.get().context("failed to get connection from pool")?;
        run_db_migrations(&mut conn);
        Ok(pool)
    }
}
