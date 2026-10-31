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

use crate::{ShopId, UserId};

/// A marker trait naming a complete store: one that implements all nine
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
{
}

/// A trait that represents a session.
///
/// All methods are `&self`: the acting session is immutable to the domain.
/// The session splits an identity half (per-browser) from a context half
/// (per-request: the active shop, ADR-0009) — how the context is carried
/// (the web layer's `X-Active-Shop` header) is not a domain concern.
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
