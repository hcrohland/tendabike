use std::ops::Deref;

use crate::domain::AnyResult;

use super::schema;
use diesel::prelude::*;
use chrono::{DateTime, Utc};

pub type AppConn = PgConnection;
pub struct Store (PooledConnection<ConnectionManager<PgConnection>>);

impl Deref for Store {
    type Target = diesel::PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Store {
    pub fn new(pool: DbPool) -> AnyResult<Self> {
        let conn = pool.get()?;
        Ok(Store(conn))
    }
}

embed_migrations!("src/s_diesel/migrations");

pub fn run_db_migrations (conn: &AppConn) {
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
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn get_connection_pool() -> DbPool {
    let database_url = std::env::var("DB_URL").unwrap_or(
        "postgres://localhost/tendabike".to_string());
        
    println!("Connecting to database {}...", database_url);
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool.")
}