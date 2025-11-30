use crate::{SqlxConn, into_domain};
use tb_domain::{Store, TbResult};

mod activity;
mod attachment;
mod garage;
mod part;
mod service;
mod serviceplan;
mod usage;
mod user;

#[async_trait::async_trait]
impl<'c> Store for SqlxConn<'c> {
    async fn commit(self) -> TbResult<()> {
        self.into_inner()
            .commit()
            .await
            .map_err(into_domain)
            .map(|_| ())
    }
}
