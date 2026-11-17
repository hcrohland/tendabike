//! The provider's identity record (ADR-0014): the typed form of the domain
//! concept (a user has exactly one provider, chosen at signup and immutable).
//! The domain stays provider-ignorant (it stores no provider); the provider
//! layer owns the typing and the identity — the same dependent-owned
//! abstraction as the id prefixes under ADR-0013.
//!
//! The enum is the single home for the provider's string encodings: its wire
//! value (the client's `user.provider` field), the activity id's prefix, the
//! web route prefix, and the OAuth entry URL. A third provider is a variant
//! carrying all four encodings at once.

use crate::facade::{Error, TbResult, UserId};

#[cfg(any(feature = "strava", feature = "icu"))]
use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope};

use crate::ProviderActivityId;

/// The provider's identity: the typed form of the domain concept (a user has
/// exactly one provider, chosen at signup and immutable). The domain stores
/// no provider; the provider layer owns the typing and derives the identity
/// from the provider-internal user data (ADR-0014).
///
/// The enum is the single home for the provider's string encodings: its wire
/// value (the client's `user.provider` field), the activity id's prefix, the
/// web route prefix, and the OAuth entry URL.
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
    /// the driver's client id, redirect URI, and scopes. `code_verifier` is
    /// the opaque PKCE code verifier the state carries (intervals.icu-only —
    /// the icu arm derives the authorize URL's S256 challenge from it; the
    /// Strava flow is a plain authorization-code exchange and passes `None`).
    #[cfg(any(feature = "strava", feature = "icu"))]
    pub fn authorize_url(&self, state: &str, code_verifier: Option<&str>) -> String {
        match self {
            #[cfg(feature = "strava")]
            Provider::Strava => {
                let (auth_url, _) = crate::strava::client::strava_client()
                    .authorize_url(|| CsrfToken::new(state.to_string()))
                    .add_scope(Scope::new(crate::strava::client::SCOPES.to_string()))
                    .url();
                auth_url.to_string()
            }
            #[cfg(feature = "icu")]
            Provider::Icu => {
                let mut builder = crate::icu::client::icu_client()
                    .authorize_url(|| CsrfToken::new(state.to_string()))
                    .add_scope(Scope::new(crate::icu::client::SCOPES.to_string()));
                if let Some(verifier) = code_verifier {
                    let challenge = PkceCodeChallenge::from_code_verifier_sha256(
                        &PkceCodeVerifier::new(verifier.to_string()),
                    );
                    builder = builder.set_pkce_challenge(challenge);
                }
                let (auth_url, _) = builder.url();
                auth_url.to_string()
            }
            #[cfg(not(all(feature = "strava", feature = "icu")))]
            _ => unreachable!("provider feature disabled"),
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

/// The user's provider, derived from the provider-internal user data
/// (ADR-0014): a row in the Strava user table means Strava, a row in the icu
/// user table means icu (a user has exactly one). The single home for "which
/// provider does this user have?" — the direct-op dispatch, the unified lane,
/// and the web layer's `GET /user` response all call it. A user with no row in
/// either table is a `NotFound`; any other read failure is a DB failure the
/// caller propagates.
#[cfg(all(feature = "strava", feature = "icu"))]
pub async fn provider_of<S>(store: &mut S, user_id: UserId) -> TbResult<Provider>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore + Send + Sync,
{
    match store.stravauser_get_by_tbid(user_id).await {
        Ok(_) => Ok(Provider::Strava),
        Err(Error::NotFound(_)) => match store.icuser_get_by_tbid(user_id).await {
            Ok(_) => Ok(Provider::Icu),
            Err(e) => Err(e),
        },
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
