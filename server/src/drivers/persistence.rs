use diesel::prelude::*;
use chrono::{DateTime, Utc};

pub mod schema;

pub type AppConn = diesel::PgConnection;

embed_migrations!("src/drivers/persistence/migrations");

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
