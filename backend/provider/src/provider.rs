//! The provider's identity record (ADR-0014): the typed form of the domain
//! concept (a user has exactly one provider, chosen at signup and immutable).
//! The domain stays provider-ignorant (it stores the string); the provider
//! layer owns the typing — the same dependent-owned abstraction as the id
//! prefixes under ADR-0013.
//!
//! The enum is the single home for the provider's string encodings: its value
//! in the `users.provider` column, the activity id's prefix, the web route
//! prefix, and the OAuth entry URL. A third provider is a variant carrying all
//! four encodings at once.

use tb_domain::{Error, TbResult, UserId};

#[cfg(any(feature = "strava", feature = "icu"))]
use oauth2::{CsrfToken, Scope};

use crate::EntityFacade;
use crate::ProviderActivityId;

/// The provider's identity: the typed form of the domain concept (a user has
/// exactly one provider, chosen at signup and immutable). The domain stores
/// the provider as a string; the provider layer owns the typing (ADR-0014).
///
/// The enum is the single home for the provider's string encodings: its value
/// in the `users.provider` column, the activity id's prefix, the web route
/// prefix, and the OAuth entry URL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provider {
    Strava,
    Icu,
}

impl Provider {
    /// The provider's value in the `users.provider` column.
    pub fn db_value(&self) -> &'static str {
        match self {
            Provider::Strava => "strava",
            Provider::Icu => "icu",
        }
    }

    /// The activity id's prefix: the `strava:` / `icu:` the domain's opaque
    /// `ActivityId` is prefixed with (the single home for the prefix
    /// knowledge, ADR-0013).
    pub fn activity_id_prefix(&self) -> &'static str {
        match self {
            Provider::Strava => "strava:",
            Provider::Icu => "icu:",
        }
    }

    /// The web route prefix: the axum nest site's path for the provider's
    /// route table.
    pub fn route_prefix(&self) -> &'static str {
        match self {
            Provider::Strava => "/strava",
            Provider::Icu => "/icu",
        }
    }

    /// The OAuth entry URL, assembled from the driver's client config and the
    /// CSRF state the web shell generates. The web shell generates the state
    /// (beside its other protocol secrets) and passes it here; the URL carries
    /// the driver's client id, redirect URI, and scopes.
    #[cfg(any(feature = "strava", feature = "icu"))]
    pub fn authorize_url(&self, state: &str) -> String {
        match self {
            #[cfg(feature = "strava")]
            Provider::Strava => {
                let (auth_url, _) = crate::strava::client::STRAVACLIENT
                    .authorize_url(|| CsrfToken::new(state.to_string()))
                    .add_scope(Scope::new(crate::strava::client::SCOPES.to_string()))
                    .url();
                auth_url.to_string()
            }
            #[cfg(feature = "icu")]
            Provider::Icu => {
                let (auth_url, _) = crate::icu::client::ICU_CLIENT
                    .authorize_url(|| CsrfToken::new(state.to_string()))
                    .add_scope(Scope::new(crate::icu::client::SCOPES.to_string()))
                    .url();
                auth_url.to_string()
            }
            #[cfg(not(all(feature = "strava", feature = "icu")))]
            _ => unreachable!("provider feature disabled"),
        }
    }
}

impl TryFrom<&str> for Provider {
    type Error = Error;

    /// Parses the provider's value in the `users.provider` column:
    /// `"strava"` → [`Provider::Strava`], `"icu"` → [`Provider::Icu`]. An
    /// unrecognized value is a `BadRequest`.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "strava" => Ok(Provider::Strava),
            "icu" => Ok(Provider::Icu),
            _ => Err(Error::BadRequest(format!("unknown provider: {s}"))),
        }
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

/// The user's provider, read from the `users.provider` column (through the
/// entity-op facade's [`EntityFacade::user_provider`] read — the owning
/// entity's op, ADR-0012) and parsed into the typed [`Provider`] (the single
/// dispatch mechanism, ADR-0014) — the same identity rule the unified lane
/// and the dispatch use. A read failure is a DB failure the caller propagates.
pub(crate) async fn provider_of<S, F: EntityFacade<S>>(
    store: &mut S,
    user_id: UserId,
    facade: &F,
) -> TbResult<Provider>
where
    S: Send + Sync,
{
    let s = facade.user_provider(&user_id, store).await?;
    Provider::try_from(s.as_str())
}

#[cfg(test)]
mod tests {
    use super::Provider;

    /// The db value matches the string literal in use today (the
    /// `users.provider` column).
    #[test]
    fn db_value_matches_current_literals() {
        assert_eq!(Provider::Strava.db_value(), "strava");
        assert_eq!(Provider::Icu.db_value(), "icu");
    }

    /// The parse round-trips the db value and rejects an unknown provider.
    #[test]
    fn parse_round_trips_and_rejects_unknown() {
        assert_eq!(Provider::try_from("strava").unwrap(), Provider::Strava);
        assert_eq!(Provider::try_from("icu").unwrap(), Provider::Icu);
        assert!(Provider::try_from("garmin").is_err());
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
