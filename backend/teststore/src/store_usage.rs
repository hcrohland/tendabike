use async_trait::async_trait;
use tb_domain::{TbResult, Usage, UsageId, UsageStore};

use super::TestStore;

#[async_trait]
impl UsageStore for TestStore {
    async fn get(&mut self, uid: UsageId) -> TbResult<Option<Usage>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        Ok(d.usages.get(&uid).cloned())
    }

    async fn update<U>(&mut self, usage: &[U]) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync,
    {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        for u in usage {
            let u = u.borrow();
            d.usages.insert(u.id, u.clone());
        }
        Ok(usage.len())
    }

    async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        match d.usages.remove(&usage) {
            Some(x) => Ok(x),
            None => Err(tb_domain::Error::NotFound(format!(
                "Usage {} not found",
                usage
            ))),
        }
    }

    async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        let mut count = 0;
        for u in usages {
            if d.usages.remove(&u.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_all(&mut self) -> TbResult<usize> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        let res = d.usages.len();
        d.usages.clear();
        Ok(res)
    }
}
