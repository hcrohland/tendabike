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

mod partnote;
pub use partnote::*;

mod usage;
pub use usage::*;

mod service;
pub use service::*;

mod serviceplan;
pub use serviceplan::*;

use crate::{ShopId, TbResult, UserId};

/// A marker trait naming a complete store: one that implements all nine
/// sub-traits. It carries no methods — the transaction lifecycle
/// (`begin`/`commit`/`rollback`) is inherent on the concrete store types and
/// is driven only by the web layer and the tests, never by the domain.
pub trait Store:
    Send
    + PartStore
    + UserStore
    + ShopStore
    + ActivityStore
    + AttachmentStore
    + PartNoteStore
    + UsageStore
    + ServiceStore
    + ServicePlanStore
{
}

/// A trait that represents a session.
pub trait Session: Send + Sync {
    fn user_id(&self) -> UserId;
    fn shop(&self) -> Option<ShopId>;
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()>;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        self.user_id().check_owner(owner, error)
    }
}
