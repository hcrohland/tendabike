//! Test support for `tb_provider` (spec #781).
//!
//! The [`MockEntityFacade`] double: the `mockall` `#[automock]` witness for
//! [`EntityFacade`], available under the `test-support` feature (and `cfg(test)`).
//! Provider tests mock this seam instead of the concrete domain ops — the
//! domain logic runs for real behind the concrete [`EntityFacadeImpl`].
//!
//! [`EntityFacade`]: crate::EntityFacade
//! [`EntityFacadeImpl`]: crate::EntityFacadeImpl

pub use crate::facade::MockEntityFacade;
