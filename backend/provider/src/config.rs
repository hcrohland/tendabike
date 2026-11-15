//! The provider configuration: all the credentials and endpoints the
//! provider drivers need, built once at the composition root and injected
//! into the crate. The internal mechanism (a `OnceLock` that the driver
//! client statics read from) is this module's implementation detail.

use std::sync::OnceLock;

/// All provider-related configuration (Strava + intervals.icu). Built by
/// the composition root from environment variables and passed to
/// [`init`] before any provider function is called.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub strava_client_id: String,
    pub strava_client_secret: String,
    pub strava_redirect_url: String,
    pub icu_client_id: String,
    pub icu_client_secret: String,
    pub icu_redirect_url: String,
    pub icu_webhook_secret: String,
}

static CONFIG: OnceLock<ProviderConfig> = OnceLock::new();

/// Initialize the provider configuration. Must be called exactly once,
/// before any provider function that needs the config.
pub fn init(config: ProviderConfig) {
    CONFIG
        .set(config)
        .expect("ProviderConfig already initialized");
}

/// The initialized config (panics if [`init`] has not been called).
pub(crate) fn get() -> &'static ProviderConfig {
    CONFIG
        .get()
        .expect("ProviderConfig not initialized — call tb_provider::init first")
}
