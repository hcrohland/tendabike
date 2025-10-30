use crate::SqlxConn;
use tb_domain::Store;
use time::OffsetDateTime;

mod activity;
mod attachment;
mod part;
mod service;
mod serviceplan;
mod usage;
mod user;

#[async_session::async_trait]
impl Store for SqlxConn {}

pub(crate) async fn migrate(
    _conn: &mut SqlxConn,
    _time: OffsetDateTime,
) -> Result<(), sqlx::Error> {
    Ok(())
}
