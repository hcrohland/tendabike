use crate::AsyncDieselConn;
use tb_domain::Store;

mod activity;
mod attachment;
mod part;
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
