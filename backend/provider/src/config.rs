//! The provider configuration: all the credentials and endpoints the
//! provider drivers need, built once at the composition root and injected
//! into the crate. The internal mechanism (a `OnceLock` that the driver
//! client statics read from) is this module's implementation detail.

use std::sync::OnceLock;

use crate::facade::Error;

/// All provider-related configuration (Strava + intervals.icu). Built by
/// the composition root from environment variables and passed to
/// [`init`] before any provider function is called.
///
/// The per-provider credential fields are optional (ADR-0016): a provider
/// is enabled iff its primary credential (client id) is set — a missing
/// client id yields a disabled provider (a graceful freeze), not a startup
/// failure.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub strava_client_id: Option<String>,
    pub strava_client_secret: Option<String>,
    pub strava_redirect_url: String,
    pub icu_client_id: Option<String>,
    pub icu_client_secret: Option<String>,
    pub icu_redirect_url: String,
    pub icu_webhook_secret: String,
}

static CONFIG: OnceLock<ProviderConfig> = OnceLock::new();
static ENABLED: OnceLock<Vec<Provider>> = OnceLock::new();

/// Initialize the provider configuration. Must be called exactly once,
/// before any provider function that needs the config. Derives the enabled
/// providers (ADR-0016) from the config's client ids.
pub fn init(config: ProviderConfig) {
    let enabled = derive_enabled(&config);
    CONFIG
        .set(config)
        .expect("ProviderConfig already initialized");
    ENABLED
        .set(enabled)
        .expect("enabled providers already initialized");
}

/// The enabled providers (ADR-0016): the runtime, ordered subset of
/// [`Provider`] (the enum's definition order) derived from the config at
/// [`init`].
pub fn enabled_providers() -> &'static [Provider] {
    ENABLED
        .get()
        .expect("ProviderConfig not initialized — call tb_provider::init first")
}

/// The clear error a disabled provider's op returns (ADR-0016): the
/// message names the provider and says it is disabled on this server.
pub(crate) fn disabled(provider: Provider) -> Error {
    Error::Unavailable(format!(
        "the {} provider is disabled on this server",
        provider.wire_value()
    ))
}

/// The enabled providers of a config: a provider is enabled iff its client
/// id is set (ADR-0016), in the `Provider` enum's definition order.
fn derive_enabled(config: &ProviderConfig) -> Vec<Provider> {
    let mut enabled = Vec::new();
    if config.strava_client_id.is_some() {
        enabled.push(Provider::Strava);
    }
    if config.icu_client_id.is_some() {
        enabled.push(Provider::Icu);
    }
    enabled
}

/// The initialized config (panics if [`init`] has not been called).
pub(crate) fn get() -> &'static ProviderConfig {
    CONFIG
        .get()
        .expect("ProviderConfig not initialized — call tb_provider::init first")
}

use crate::Provider;

#[cfg(test)]
mod tests {
    use super::*;

    fn config(strava_client_id: Option<&str>, icu_client_id: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            strava_client_id: strava_client_id.map(String::from),
            strava_client_secret: strava_client_id.map(|_| "strava-secret".to_string()),
            strava_redirect_url: "http://localhost:8000/strava/token".to_string(),
            icu_client_id: icu_client_id.map(String::from),
            icu_client_secret: icu_client_id.map(|_| "icu-secret".to_string()),
            icu_redirect_url: "http://localhost:8000/icu/token".to_string(),
            icu_webhook_secret: String::new(),
        }
    }

    /// A provider is enabled iff its client id is set (ADR-0016); the set
    /// is in the `Provider` enum's definition order.
    #[test]
    fn both_client_ids_set_enables_both_in_enum_order() {
        let enabled = derive_enabled(&config(Some("sid"), Some("iid")));
        assert_eq!(enabled, vec![Provider::Strava, Provider::Icu]);
    }

    #[test]
    fn only_strava_client_id_set_enables_only_strava() {
        let enabled = derive_enabled(&config(Some("sid"), None));
        assert_eq!(enabled, vec![Provider::Strava]);
    }

    #[test]
    fn only_icu_client_id_set_enables_only_icu() {
        let enabled = derive_enabled(&config(None, Some("iid")));
        assert_eq!(enabled, vec![Provider::Icu]);
    }

    #[test]
    fn no_client_id_set_enables_nothing() {
        let enabled = derive_enabled(&config(None, None));
        assert!(enabled.is_empty());
    }

    /// The disabled-provider error names the provider and says it is
    /// disabled on this server (ADR-0016) — the clear message a frozen
    /// provider's failing ops carry.
    #[test]
    fn disabled_error_names_the_provider() {
        for (provider, wire) in [(Provider::Strava, "strava"), (Provider::Icu, "icu")] {
            let err = disabled(provider);
            assert!(
                matches!(&err, Error::Unavailable(msg) if msg.contains(wire) && msg.contains("disabled on this server")),
                "the {wire} disabled error must name the provider and state the freeze: {err:?}"
            );
        }
    }
}
