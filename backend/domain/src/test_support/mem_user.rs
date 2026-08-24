use super::*;
use crate::{OnboardingStatus, TbResult, User, UserId};

#[async_trait::async_trait]
impl UserStore for MemStore {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        todo!()
    }

    async fn create(
        &mut self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        todo!()
    }

    async fn update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        todo!()
    }

    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> {
        todo!()
    }

    async fn update_onboarding_status(
        &mut self,
        uid: &UserId,
        status: OnboardingStatus,
    ) -> TbResult<User> {
        todo!()
    }
}
