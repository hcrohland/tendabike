//! Test support for the icu driver (spec #763).
//!
//! - `test-support` feature → exposes [`MockIcuDataStore`] to consumer crates
//!   (`tb_exec`, `tb_sqlx`).
//! - `#[cfg(test)]` → exposes [`MockIcuDataStore`] (the `automock` witness)
//!   and the fixture constructors to the icu module's own tests.

#[cfg(any(test, feature = "test-support"))]
pub use super::traits::MockIcuDataStore;

use super::{IcuId, IcuUser};
use tb_domain::{OnboardingStatus, User, UserId};

/// Builds an [`IcuUser`] for tests.
pub fn icu_user(tbid: UserId, icuathlete: &str, enabled: bool) -> IcuUser {
    IcuUser {
        id: icuathlete.into(),
        tendabike_id: tbid,
        access_token: enabled.then(|| "test-token".to_string()),
        disabled: !enabled,
    }
}

/// A canned [`User`] with the given id (the icu provider, per spec #763).
pub fn user(id: UserId) -> User {
    User {
        id,
        name: "Test User".to_string(),
        firstname: "Test".to_string(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Completed,
        provider: "icu".to_string(),
    }
}

/// The standard test triple: a fresh `MockIcuDataStore`, a
/// `MockDomainFacade` (with `C` = `MockIcuDataStore`), and nothing else —
/// the icu lane has no acting session of its own.
#[cfg(test)]
pub fn setup() -> (
    super::traits::MockIcuDataStore,
    crate::MockDomainFacade<super::traits::MockIcuDataStore>,
) {
    let store = super::traits::MockIcuDataStore::new();
    let facade = crate::MockDomainFacade::<super::traits::MockIcuDataStore>::new();
    (store, facade)
}

/// A canned icu athlete id.
pub fn icu_id(n: &str) -> IcuId {
    n.into()
}
