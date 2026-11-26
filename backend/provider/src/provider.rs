//! The provider's identity record (ADR-0014): the typed form of the domain
//! concept (a user has exactly one provider, chosen at signup and immutable).
//! The domain stays provider-ignorant (it stores no provider); the provider
//! layer owns the typing and the identity — the same dependent-owned
//! abstraction as the id prefixes under ADR-0013.
//!
//! The enum is the single home for the provider's string encodings: its wire
//! value (the client's `user.provider` field), with the activity id's prefix
//! and the web route prefix derived from it. The OAuth authorize URL lives on
//! the driver, not here. A third provider is a variant carrying the wire
//! value; the prefixes derive from it.

use crate::facade::{Error, TbResult, UserId};

use crate::ProviderActivityId;

/// The provider's identity: the typed form of the domain concept (a user has
/// exactly one provider, chosen at signup and immutable). The domain stores
/// no provider; the provider layer owns the typing and derives the identity
/// from the provider-internal user data (ADR-0014).
///
/// The enum is the single home for the provider's string encodings: its wire
/// value (the client's `user.provider` field), with the activity id's prefix
/// and the web route prefix derived from it. The OAuth authorize URL lives on
/// the driver, not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provider {
    Strava,
    Icu,
}

impl Provider {
    /// The provider's wire value: the string the client's `user.provider`
    /// field carries (`"strava"` | `"icu"`).
    pub fn wire_value(&self) -> &'static str {
        match self {
            Provider::Strava => "strava",
            Provider::Icu => "icu",
        }
    }

    /// The activity id's prefix: the `strava:` / `icu:` the domain's opaque
    /// `ActivityId` is prefixed with — derived from the wire value (the
    /// single home for the prefix knowledge, ADR-0013).
    pub fn activity_id_prefix(&self) -> String {
        format!("{}:", self.wire_value())
    }

    /// The web route prefix: the axum nest site's path for the provider's
    /// route table — derived from the wire value.
    pub fn route_prefix(&self) -> String {
        format!("/{}", self.wire_value())
    }
}

impl TryFrom<ProviderActivityId> for Provider {
    type Error = Error;

    /// The provider an activity id belongs to: [`ProviderActivityId::Strava`]
    /// → [`Provider::Strava`], [`ProviderActivityId::Icu`] →
    /// [`Provider::Icu`]. The mapping is total (the enum's variants are the
    /// provider's variants).
    fn try_from(id: ProviderActivityId) -> Result<Self, Self::Error> {
        match id {
            ProviderActivityId::Strava(_) => Ok(Provider::Strava),
            ProviderActivityId::Icu(_) => Ok(Provider::Icu),
        }
    }
}

/// The user's provider, derived from the provider-internal user data
/// (ADR-0014): a row in the Strava user table means Strava, a row in the icu
/// user table means icu (a user has exactly one). The single home for "which
/// provider does this user have?" — the direct-op dispatch, the unified lane,
/// and the web layer's `GET /user` response all call it. A user with no row in
/// either table is a `NotFound`; any other read failure is a DB failure the
/// caller propagates.
#[cfg(all(feature = "strava", feature = "icu"))]
pub async fn provider_of<S: crate::ProviderStore>(
    store: &mut S,
    user_id: UserId,
) -> TbResult<Provider> {
    match crate::store::Store::<crate::strava::StravaSpec>::user_get_by_tbid(store, user_id).await {
        Ok(_) => Ok(Provider::Strava),
        Err(Error::NotFound(_)) => {
            match crate::store::Store::<crate::icu::IcuSpec>::user_get_by_tbid(store, user_id).await
            {
                Ok(_) => Ok(Provider::Icu),
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;

    /// The wire value matches the string literal the client's `user.provider`
    /// field carries.
    #[test]
    fn wire_value_matches_current_literals() {
        assert_eq!(Provider::Strava.wire_value(), "strava");
        assert_eq!(Provider::Icu.wire_value(), "icu");
    }

    /// The activity id prefix matches the string literal in use today (the
    /// `strava:` / `icu:` the domain's opaque id is prefixed with).
    #[test]
    fn activity_id_prefix_matches_current_literals() {
        assert_eq!(Provider::Strava.activity_id_prefix(), "strava:");
        assert_eq!(Provider::Icu.activity_id_prefix(), "icu:");
    }

    /// The route prefix matches the string literal in use today (the axum
    /// nest site's path).
    #[test]
    fn route_prefix_matches_current_literals() {
        assert_eq!(Provider::Strava.route_prefix(), "/strava");
        assert_eq!(Provider::Icu.route_prefix(), "/icu");
    }
}
