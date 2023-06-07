use crate::{AnyResult, User, UserId};

#[async_trait::async_trait]
pub trait UserStore {
    async fn user_read_by_id(&mut self, uid: UserId) -> AnyResult<User>;

    async fn user_create(&mut self, firstname_: &str, lastname: &str) -> AnyResult<User>;

    async fn user_update(
        &mut self,
        uid: &UserId,
        firstname_: &str,
        lastname: &str,
    ) -> AnyResult<User>;
}
