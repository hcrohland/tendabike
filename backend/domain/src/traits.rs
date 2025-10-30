mod part;
pub use part::*;

mod user;
pub use user::*;

mod activity;
pub use activity::*;

mod attachment;
pub use attachment::*;

mod usage;
pub use usage::*;

mod service;
pub use service::*;

mod serviceplan;
pub use serviceplan::*;

use crate::UserId;

#[async_trait::async_trait]
/// A trait that represents a store for various tb_domain models.
pub trait Store:
    Send
    + PartStore
    + UserStore
    + ActivityStore
    + AttachmentStore
    + UsageStore
    + ServiceStore
    + ServicePlanStore
{
}

/// A trait that represents a person.
pub trait Person: Send + Sync {
    fn get_id(&self) -> UserId;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        if self.get_id() == owner {
            Ok(())
        } else {
            Err(crate::Error::Forbidden(error))
        }
    }
}
