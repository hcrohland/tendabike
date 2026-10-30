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

mod notification;
pub use notification::*;

use crate::{ShopId, UserId};

/// A marker trait naming a complete store: one that implements all ten
/// sub-traits. It carries no methods — the transaction lifecycle
/// (`begin`/`commit`/`rollback`) is inherent on the concrete store types and
/// is driven only by the callers (the web layer, the loop, and the tests),
/// never by the domain.
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
    + NotificationStore
{
}

/// A trait that represents a session.
///
/// All methods are `&self`: the acting session is immutable to the domain.
/// The web layer's cookie persistence for the active shop is not a domain
/// concern — it stays on the concrete request session as an inherent method
/// (spec #517, ticket #588).
///
/// `#[automock]` under the `test-support` feature generates a
/// [`MockSession`]: one mockall double replaces every test's hand-rolled
/// `Session` fake.
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait Session: Send + Sync {
    fn user_id(&self) -> UserId;
    fn shop(&self) -> Option<ShopId>;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        self.user_id().check_owner(owner, error)
    }
}
