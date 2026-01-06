mod part;
pub use part::*;

mod user;
pub use user::*;

mod shop;
pub use shop::*;

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

use crate::{ShopId, TbResult, UserId};

#[async_trait::async_trait]
/// A trait that represents a store for various tb_domain models.
pub trait Store:
    Send
    + PartStore
    + UserStore
    + ShopStore
    + ActivityStore
    + AttachmentStore
    + UsageStore
    + ServiceStore
    + ServicePlanStore
{
    async fn commit(self) -> TbResult<()>;
}

/// A trait that represents a session.
pub trait Session: Send + Sync {
    fn get_id(&self) -> UserId;
    fn shop(&self) -> Option<ShopId>;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        if self.get_id() == owner {
            Ok(())
        } else {
            Err(crate::Error::Forbidden(error))
        }
    }
}
