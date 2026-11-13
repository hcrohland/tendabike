//! The provider layer's typed activity id (ADR-0013): the cross-provider form
//! the domain's opaque [`ActivityId`] converts to and from, and the single
//! home for the `strava:` / `icu:` prefix knowledge.
//!
//! The domain's [`ActivityId`] is an opaque string — the domain never names a
//! provider (ADR-0013). The prefix is a provider concern, so it lives here:
//! [`ProviderActivityId::try_from`] parses the opaque id into the
//! provider-typed enum, and [`ActivityId::from`] composes it back. The
//! executor's dispatch matches on the enum; the drivers build their URLs from
//! the raw (unprefixed) ids.

use derive_more::{Display, From, Into};
use tb_domain::{ActivityId, Error};

/// The Strava activity id: the raw numeric Strava id (unprefixed). The inner
/// value is private — compare/convert with `i64::from(id)` /
/// `StravaActivityId::from(n)`.
#[derive(Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq)]
pub struct StravaActivityId(i64);

/// The intervals.icu activity id: the raw icu text id (unprefixed). The inner
/// value is private — compare/convert with `String::from(id)` /
/// `IcuActivityId::from(s)`.
#[derive(Clone, Debug, Display, From, Into, Hash, PartialEq, Eq)]
#[display("{}", _0)]
pub struct IcuActivityId(String);

impl From<&str> for IcuActivityId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// The provider layer's typed activity id: the cross-provider form the
/// domain's opaque [`ActivityId`] converts to and from. The `strava:` /
/// `icu:` prefix knowledge lives nowhere else (ADR-0013).
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ProviderActivityId {
    Strava(StravaActivityId),
    Icu(IcuActivityId),
}

impl TryFrom<ActivityId> for ProviderActivityId {
    type Error = Error;

    /// Parses the opaque domain id into its provider-typed form:
    /// `"strava:<n>"` → [`ProviderActivityId::Strava`], `"icu:<id>"` →
    /// [`ProviderActivityId::Icu`]. This is the single home for the prefix
    /// knowledge (ADR-0013). An unrecognized prefix is a `BadRequest`
    /// (matching the icu driver's `not an icu activity` behavior).
    fn try_from(id: ActivityId) -> Result<Self, Self::Error> {
        let s: String = id.into();
        if let Some(n) = s.strip_prefix("strava:") {
            n.parse::<i64>()
                .map(|n| Self::Strava(StravaActivityId::from(n)))
                .map_err(|_| Error::BadRequest(format!("not a strava activity: {s}")))
        } else if let Some(id) = s.strip_prefix("icu:") {
            Ok(Self::Icu(IcuActivityId::from(id)))
        } else {
            Err(Error::BadRequest(format!("not an icu activity: {s}")))
        }
    }
}

impl From<ProviderActivityId> for ActivityId {
    /// Composes the opaque domain id: `Strava(n)` → `"strava:<n>"`,
    /// `Icu(id)` → `"icu:<id>"`. This is the single home for the prefix
    /// composition.
    fn from(id: ProviderActivityId) -> Self {
        match id {
            ProviderActivityId::Strava(n) => ActivityId::from(format!("strava:{n}")),
            ProviderActivityId::Icu(id) => ActivityId::from(format!("icu:{id}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use tb_domain::{ActivityId, Error};

    use super::{IcuActivityId, ProviderActivityId, StravaActivityId};

    /// Round-trip: the opaque id parses to the provider-typed form and
    /// composes back to the same string (Strava).
    #[test]
    fn strava_round_trip() {
        let id = ActivityId::from("strava:12345");
        let typed = ProviderActivityId::try_from(id.clone()).unwrap();
        assert_eq!(
            typed,
            ProviderActivityId::Strava(StravaActivityId::from(12345))
        );
        assert_eq!(ActivityId::from(typed), id);
    }

    /// Round-trip: the opaque id parses to the provider-typed form and
    /// composes back to the same string (icu).
    #[test]
    fn icu_round_trip() {
        let id = ActivityId::from("icu:i55751783");
        let typed = ProviderActivityId::try_from(id.clone()).unwrap();
        assert_eq!(
            typed,
            ProviderActivityId::Icu(IcuActivityId::from("i55751783"))
        );
        assert_eq!(ActivityId::from(typed), id);
    }

    /// An unrecognized prefix is a `BadRequest` (not a provider id).
    #[test]
    fn try_from_rejects_unknown_prefix() {
        let res = ProviderActivityId::try_from(ActivityId::from("garmin:42"));
        assert!(matches!(res, Err(Error::BadRequest(_))));
    }
}
