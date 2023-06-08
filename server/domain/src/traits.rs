
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
/// A trait that represents a store for various domain models.
pub trait Store:
    diesel_async::AsyncConnection
    + TypesStore
    + PartStore
    + UserStore
    + ActivityStore
    + AttachmentStore
{}

/// A trait that represents a person.
pub trait Person: Send + Sync {
    fn get_id(&self) -> UserId;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::AnyResult<()> {
        if self.get_id() == owner || self.is_admin() {
            Ok(())
        } else {
            Err(crate::Error::Forbidden(error).into())
        }
    }
}
