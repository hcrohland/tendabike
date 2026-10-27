use super::*;
use crate::{TbResult, Usage, UsageId};

#[async_trait::async_trait]
impl UsageStore for MemStore {
    async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>> {
        let d = self.state();
        Ok(d.usages.get(&uid).cloned())
    }

    async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize> {
        let mut d = self.state();
        for u in usage {
            d.usages.insert(u.id, u.clone());
        }
        Ok(usage.len())
    }

    async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        let mut d = self.state();
        match d.usages.remove(&usage) {
            Some(x) => Ok(x),
            None => Err(crate::Error::NotFound(format!("Usage {} not found", usage))),
        }
    }

    async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize> {
        let mut d = self.state();
        let mut count = 0;
        for u in usages {
            if d.usages.remove(&u.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        let mut d = self.state();
        let res = d.usages.len();
        d.usages.clear();
        Ok(res)
    }
}
