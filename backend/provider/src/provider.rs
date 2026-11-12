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

use tb_domain::Error;

#[cfg(any(feature = "strava", feature = "icu"))]
use oauth2::{CsrfToken, Scope};

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

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::Provider;

    /// Dummies the OAuth client environment variables so the `STRAVACLIENT`
    /// and `ICU_CLIENT` lazy statics do not panic when they are first used in
    /// a test. Only sets a variable that is not already present, so a real
    /// environment's values take precedence.
    fn set_oauth_env_once() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| unsafe {
            for (key, val) in [
                ("CLIENT_ID", "test-client-id"),
                ("CLIENT_SECRET", "test-client-secret"),
                ("ICU_CLIENT_ID", "test-icu-client-id"),
                ("ICU_CLIENT_SECRET", "test-icu-client-secret"),
            ] {
                if std::env::var(key).is_err() {
                    std::env::set_var(key, val);
                }
            }
        });
    }

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

    /// The Strava authorize URL matches the shape in use today: the auth
    /// endpoint, the client id, the CSRF state, and the scopes.
    #[test]
    fn strava_authorize_url_matches_current_shape() {
        set_oauth_env_once();
        let url = Provider::Strava.authorize_url("test-state");
        assert!(
            url.starts_with("https://www.strava.com/oauth/authorize?"),
            "unexpected url {url}"
        );
        assert!(url.contains("client_id="), "missing client_id in {url}");
        assert!(url.contains("state=test-state"), "missing state in {url}");
        assert!(url.contains("scope="), "missing scope in {url}");
    }

    /// The icu authorize URL matches the shape in use today: the auth
    /// endpoint, the client id, the CSRF state, and the scopes.
    #[test]
    fn icu_authorize_url_matches_current_shape() {
        set_oauth_env_once();
        let url = Provider::Icu.authorize_url("test-state");
        assert!(
            url.starts_with("https://intervals.icu/oauth/authorize?"),
            "unexpected url {url}"
        );
        assert!(url.contains("client_id="), "missing client_id in {url}");
        assert!(url.contains("state=test-state"), "missing state in {url}");
        assert!(url.contains("scope="), "missing scope in {url}");
    }
}
