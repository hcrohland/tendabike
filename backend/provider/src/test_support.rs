//! Test support for `tb_provider` (spec #781).
//!
//! The [`MockDomainFacade`] double: the `mockall` `#[automock]` witness for
//! [`DomainFacade`], available under the `test-support` feature (and `cfg(test)`).
//! Provider tests mock this seam instead of the concrete domain ops — the
//! domain logic runs for real behind the concrete [`DomainFacadeImpl`].
//!
//! [`DomainFacade`]: crate::DomainFacade
//! [`DomainFacadeImpl`]: crate::DomainFacadeImpl

pub use crate::facade::MockDomainFacade;
