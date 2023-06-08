use crate::AppConn;

use domain::Store;

mod types;
mod part;
mod user;
mod activity;
mod attachment;

#[async_session::async_trait]
impl Store for AppConn {
    async fn storetransaction<'a, R, E, F>(&mut self, callback: F) -> Result<R, E>
    where
        F: for<'r> FnOnce(&'r mut Self) -> scoped_futures::ScopedBoxFuture<'a,'r,Result<R,E> >  + Send + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
    {
        self.storetransaction(callback).await
    }
}