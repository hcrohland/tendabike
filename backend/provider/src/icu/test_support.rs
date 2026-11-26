//! Test support for the icu driver (spec #763/#764): the fixture
//! constructors and the canned-data builders. The shared canned-response
//! wiring (the thread-local map and the `begin_test` / `queue` /
//! `pop_test_canned` / `requests` accessors) lives in the kit's
//! [`crate::test_support`]; the icu driver has no provider-specific canned
//! data of its own (its tokens are durable, read straight from `icu_users`).
//!
//! The store double is the single [`MockStore`](crate::test_support::MockStore)
//! over [`IcuSpec`](super::IcuSpec) (spec #1029) — the per-provider data-store
//! mock is gone.

use oauth2::AccessToken;

use super::{IcuId, IcuUser};
use time::OffsetDateTime;

use crate::facade::{OnboardingStatus, Part, PartId, PartTypeId, UsageId, User, UserId};

/// Builds an [`IcuUser`] for tests.
pub fn icu_user(tbid: UserId, icuathlete: &str, enabled: bool) -> IcuUser {
    IcuUser {
        id: icuathlete.into(),
        tendabike_id: tbid,
        // `enabled` drives the token: a cleared token (None) is the disable
        // signal, so `IcuUser::disabled()` derives the flag from it.
        access_token: enabled.then(|| AccessToken::new("test-token".to_string())),
    }
}

/// A canned [`User`] with the given id.
pub fn user(id: UserId) -> User {
    User {
        id,
        name: "Test User".to_string(),
        firstname: "Test".to_string(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Completed,
    }
}

/// The standard test pair: a fresh [`MockStore<IcuSpec>`] and a
/// [`MockEntityFacade`] (with `C` = [`MockStore<IcuSpec>`]). The icu lane has
/// no acting session of its own, so (unlike the Strava driver's `setup`,
/// which reaches the seam through the session) this enables the HTTP seam's
/// test mode directly — the kit's `begin_test` marks the thread in test mode
/// so the canned responses the tests queue are replayed instead of the real
/// API.
#[cfg(test)]
pub fn setup() -> (
    crate::test_support::MockStore<super::IcuSpec>,
    crate::test_support::MockEntityFacade<crate::test_support::MockStore<super::IcuSpec>>,
) {
    crate::test_support::begin_test();
    let mut store = crate::test_support::MockStore::<super::IcuSpec>::new();
    store.expect_limited_until().returning(|| None);
    let facade = crate::test_support::MockEntityFacade::<
        crate::test_support::MockStore<super::IcuSpec>,
    >::new();
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

/// Minimal icu gear JSON with the given id and type (the `distance`/`retired`
/// fields the domain ignores are present, as the icu API returns them).
pub fn gear_json(id: &str, typ: &str) -> String {
    format!(
        r#"{{"id":"{id}","name":"Test Gear","type":"{typ}","distance":1000.0,"retired":false}}"#
    )
}
