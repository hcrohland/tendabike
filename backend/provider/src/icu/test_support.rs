//! Test support for the icu driver (spec #763/#764).
//!
//! - `test-support` feature → exposes [`MockIcuDataStore`] and the [`seam`]
//!   (the HTTP canned responses) to consumer crates (`tb_exec`, `tb_sqlx`).
//! - `#[cfg(test)]` → exposes [`MockIcuDataStore`] (the `automock` witness),
//!   the [`seam`], and the fixture constructors to the icu module's own tests.

pub mod seam;
pub use seam::*;

#[cfg(any(test, feature = "test-support"))]
pub use super::traits::MockIcuDataStore;

use super::{IcuId, IcuUser};
use tb_domain::{OnboardingStatus, Part, PartId, PartTypeId, UsageId, User, UserId};
use time::OffsetDateTime;

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

/// The standard test triple: a fresh `MockIcuDataStore` and a
/// `MockDomainFacade` (with `C` = `MockIcuDataStore`). The icu lane has no
/// acting session of its own, so (unlike the Strava driver's `setup`, which
/// reaches the seam through the session) this enables the HTTP seam's test
/// mode directly — `begin_test` marks the thread in test mode so the canned
/// responses the tests queue are replayed instead of the real API.
#[cfg(test)]
pub fn setup() -> (
    super::traits::MockIcuDataStore,
    crate::MockDomainFacade<super::traits::MockIcuDataStore>,
) {
    begin_test();
    let store = super::traits::MockIcuDataStore::new();
    let facade = crate::MockDomainFacade::<super::traits::MockIcuDataStore>::new();
    (store, facade)
}

/// A canned icu athlete id.
pub fn icu_id(n: &str) -> IcuId {
    n.into()
}

/// A canned [`Part`] for tests.
pub fn part(id: PartId, owner: UserId, what: PartTypeId, source: Option<String>) -> Part {
    let now = OffsetDateTime::UNIX_EPOCH;
    Part {
        id,
        owner,
        what,
        name: "Test Bike".to_string(),
        vendor: "Test Vendor".to_string(),
        model: "Test Model".to_string(),
        purchase: now,
        last_used: now,
        disposed_at: None,
        usage: UsageId::default(),
        source,
        shop: None,
    }
}

/// Minimal icu activity JSON with the given id, type, distance, moving time,
/// and gear id.
pub fn activity_json(id: &str, typ: &str, gear: Option<&str>) -> String {
    let gear = match gear {
        Some(g) => format!("{{\"id\":\"{g}\"}}"),
        None => "null".to_string(),
    };
    format!(
        r#"{{"id":"{id}","type":"{typ}","name":"Test Ride","start_date":"2026-01-02T10:00:00Z","distance":25000.0,"moving_time":3300,"elapsed_time":3600,"total_elevation_gain":300.0,"icu_joules":4000.0,"gear":{gear}}}"#
    )
}

/// A stub icu activity JSON (both distance and moving_time null).
pub fn stub_activity_json(id: &str, typ: &str) -> String {
    format!(
        r#"{{"id":"{id}","type":"{typ}","name":"Stub Ride","start_date":"2026-01-02T10:00:00Z","distance":null,"moving_time":null,"elapsed_time":0,"total_elevation_gain":null,"icu_joules":null,"gear":null}}"#
    )
}
