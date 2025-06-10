use crate::AsyncDieselConn;
use async_session::log::info;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use scoped_futures::ScopedFutureExt;
use tb_domain::{Attachment, Part, Store};
use time::OffsetDateTime;

mod activity;
pub mod attachment;
mod part;
mod service;
mod serviceplan;
mod usage;
mod user;

#[async_session::async_trait]
impl Store for AsyncDieselConn {
    async fn transaction<'a, R, E, F>(&mut self, callback: F) -> Result<R, E>
    where
        F: for<'r> FnOnce(&'r mut Self) -> scoped_futures::ScopedBoxFuture<'a, 'r, Result<R, E>>
            + Send
            + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
    {
        diesel_async::AsyncConnection::transaction(self, callback).await
    }
}

pub(crate) async fn migrate(
    conn: &mut AsyncDieselConn,
    time: OffsetDateTime,
) -> Result<(), diesel::result::Error> {
    use tb_domain::schema::attachments::dsl::*;
    use tb_domain::schema::parts::dsl::*;
    conn.transaction(|conn| {
        async move {
            if time > OffsetDateTime::now_utc() {
                let atts = attachments.get_results::<Attachment>(conn).await?;
                for attach in atts {
                    if let Some((att, det)) = attach.round_times() {
                        info!("Rounding times for {attach:?} to {att}, {det}");
                        diesel::update(attachments.find(attach.key()))
                            .set((attached.eq(att), detached.eq(det)))
                            .execute(conn)
                            .await?;
                    }
                }
                let partlist = parts.get_results::<Part>(conn).await?;
                for p in partlist {
                    let purchased = tb_domain::round_time(p.purchase);
                    if purchased != p.purchase {
                        info!("Rounding purchase for part {} to {purchased}", p.id);
                        diesel::update(parts.find(p.id))
                            .set(purchase.eq(purchased))
                            .execute(conn)
                            .await?;
                    }
                }
            }

            Ok(())
        }
        .scope_boxed()
    })
    .await
}
