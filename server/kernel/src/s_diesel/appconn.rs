use std::ops::Deref;

use crate::domain::AnyResult;

use super::schema;
use anyhow::Context;
use diesel::prelude::*;
use chrono::{DateTime, Utc};

pub type AppConn = PgConnection;
pub struct Store (PooledConnection<ConnectionManager<AppConn>>);

impl Deref for Store {
    type Target = AppConn;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Store {
    pub fn new(pool: &DbPool) -> AnyResult<Self> {
        let conn = pool.0.get()?;
        Ok(Store(conn))
    }

}


embed_migrations!("src/s_diesel/migrations");

fn run_db_migrations (conn: &AppConn) {
    use schema::attachments::dsl::*;

    diesel::update(attachments)
        .filter(detached.is_null())
        .set(detached.eq(DateTime::<Utc>::MAX_UTC))
        .execute(conn)
        .expect("rewrite detached failed");

    embedded_migrations::run(conn)
        .expect("Failed to run database migrations: {:?}");
}


use r2d2::{Pool, PooledConnection};
use diesel::r2d2::ConnectionManager;

#[derive(Clone)]
pub struct DbPool (pub r2d2::Pool<ConnectionManager<AppConn>>);

impl DbPool {
    pub fn new() -> AnyResult<Self> {
        let database_url = std::env::var("DB_URL").unwrap_or(
            "postgres://localhost/tendabike".to_string());
            
        println!("Connecting to database {}...", database_url);
        let manager = ConnectionManager::<AppConn>::new(database_url);

        let pool = Pool::builder()
            .build(manager)
            .context("Failed to create database connection pool.")?;

        let conn = pool.get().context("failed to get connection from pool")?;
        run_db_migrations(&conn);
        Ok(Self(pool))
}
}