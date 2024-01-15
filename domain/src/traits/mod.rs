mod types;
pub use types::*;

mod part;
pub use part::*;

mod user;
pub use user::*;

mod activity;
pub use activity::*;

mod attachment;
pub use attachment::*;

use crate::UserId;

#[async_trait::async_trait]
/// A trait that represents a store for various tb_domain models.
pub trait Store:
    Send + TypesStore + PartStore + UserStore + ActivityStore + AttachmentStore
{
    async fn transaction<'a, R, E, F>(&mut self, callback: F) -> Result<R, E>
    where
        F: for<'r> FnOnce(&'r mut Self) -> scoped_futures::ScopedBoxFuture<'a, 'r, Result<R, E>>
            + Send
            + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a;
}

/// A trait that represents a person.
pub trait Person: Send + Sync {
    fn get_id(&self) -> UserId;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        if self.get_id() == owner {
            Ok(())
        } else {
            Err(crate::Error::Forbidden(error).into())
        }
    }
}
