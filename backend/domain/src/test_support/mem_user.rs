use super::*;
use crate::{OnboardingStatus, TbResult, User, UserId};

#[async_trait::async_trait]
impl UserStore for MemStore {
    async fn get(&mut self, _uid: UserId) -> TbResult<User> {
        todo!()
    }

    async fn create(
        &mut self,
        _firstname: &str,
        _lastname: &str,
        _avatar: &Option<String>,
    ) -> TbResult<User> {
        todo!()
    }

    async fn update(
        &mut self,
        _uid: &UserId,
        _firstname: &str,
        _lastname: &str,
        _avatar: &Option<String>,
    ) -> TbResult<User> {
        todo!()
    }

    async fn user_delete(&mut self, _user: &UserId) -> TbResult<usize> {
        todo!()
    }

    async fn update_onboarding_status(
        &mut self,
        _uid: &UserId,
        _status: OnboardingStatus,
    ) -> TbResult<User> {
        todo!()
    }
}
