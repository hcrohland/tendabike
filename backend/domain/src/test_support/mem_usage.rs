use super::*;
use crate::{TbResult, Usage, UsageId};

#[async_trait::async_trait]
impl UsageStore for MemStore {
    async fn get(&mut self, _uid: UsageId) -> TbResult<Option<Usage>> {
        todo!()
    }

    async fn update<U>(&mut self, _usage: &[U]) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync,
    {
        todo!()
    }

    async fn delete(&mut self, _usage: UsageId) -> TbResult<Usage> {
        todo!()
    }

    async fn usages_delete(&mut self, _usages: &[Usage]) -> TbResult<usize> {
        todo!()
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        todo!()
    }
}
