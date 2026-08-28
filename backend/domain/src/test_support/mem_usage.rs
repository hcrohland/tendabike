use super::*;
use crate::{TbResult, Usage, UsageId};

#[async_trait::async_trait]
impl UsageStore for MemStore {
    async fn get(&mut self, uid: UsageId) -> TbResult<Option<Usage>> {
        Ok(self.usages.get(&uid).cloned())
    }

    async fn update<U>(&mut self, usage: &[U]) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync,
    {
        for u in usage {
            let u = u.borrow();
            self.usages.insert(u.id, u.clone());
        }
        Ok(usage.len())
    }

    async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        match self.usages.remove(&usage) {
            Some(x) => Ok(x),
            None => Err(crate::Error::NotFound(format!("Usage {} not found", usage))),
        }
    }

    async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize> {
        let mut count = 0;
        for u in usages {
            if self.usages.remove(&u.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        let res = self.usages.len();
        self.usages.clear();
        Ok(res)
    }
}
