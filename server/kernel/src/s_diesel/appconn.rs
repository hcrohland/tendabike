use std::ops::Deref;

use super::schema;
use diesel::prelude::*;
use chrono::{DateTime, Utc};

pub type AppConn = PgConnection;
pub struct Store (diesel::PgConnection);

impl Deref for Store {
    type Target = diesel::PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Store {
    pub fn new(conn: PgConnection) -> Self{
        Self(conn)
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
