//! Startup configuration: resolves the process's environment into the
//! concrete config the crates consume.
//!
//! The public host (`HOSTNAME`) is the single source for both providers'
//! redirect and webhook callback URLs. `STRAVA_*` and `DATABASE_URL` are
//! the canonical variable names; `CLIENT_ID`, `CLIENT_SECRET`, and
//! `DB_URL` remain as deprecated fallbacks — a warning is produced for
//! each one that actually supplies a value.

use std::collections::HashMap;

/// The resolved instance configuration: the database URL, both providers'
/// credentials (optional — a missing client id disables the provider,
/// ADR-0016), redirect and webhook callback URLs, and the ICU webhook
/// secret.
#[derive(Debug, Clone)]
pub(crate) struct AppConfig {
    pub(crate) database_url: String,
    pub(crate) strava_client_id: Option<String>,
    pub(crate) strava_client_secret: Option<String>,
    pub(crate) strava_redirect_url: String,
    pub(crate) strava_webhook_url: String,
    pub(crate) icu_client_id: Option<String>,
    pub(crate) icu_client_secret: Option<String>,
    pub(crate) icu_redirect_url: String,
    pub(crate) icu_webhook_url: String,
    pub(crate) icu_webhook_secret: String,
}

/// The base URL (`<scheme>://<host>`) derived from the instance's public
/// host: `http` when the host part is `localhost` or `127.0.0.1`, `https`
/// otherwise. An unset `HOSTNAME` yields the local-dev default
/// (`http://localhost:8000`).
fn base_url(hostname: Option<&str>) -> String {
    let host = hostname.unwrap_or("localhost:8000");
    let host_part = host.split(':').next().unwrap_or(host);
    let scheme = if matches!(host_part, "localhost" | "127.0.0.1") {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{host}")
}

/// The provider redirect URLs derived from the instance's public host:
/// `https://<HOSTNAME>/strava/token` and `https://<HOSTNAME>/icu/token`,
/// with `http` when the host part is `localhost` or `127.0.0.1`. An unset
/// `HOSTNAME` yields the local-dev defaults (`http://localhost:8000/…`).
struct RedirectUrls {
    strava: String,
    icu: String,
}

fn redirect_urls(hostname: Option<&str>) -> RedirectUrls {
    let base = base_url(hostname);
    RedirectUrls {
        strava: format!("{base}/strava/token"),
        icu: format!("{base}/icu/token"),
    }
}

/// The provider webhook callback URLs derived from the instance's public
/// host: `https://<HOSTNAME>/strava/callback` and
/// `https://<HOSTNAME>/icu/callback`, with `http` when the host part is
/// `localhost` or `127.0.0.1`. An unset `HOSTNAME` yields the local-dev
/// defaults (`http://localhost:8000/…`).
struct WebhookUrls {
    strava: String,
    icu: String,
}

fn webhook_urls(hostname: Option<&str>) -> WebhookUrls {
    let base = base_url(hostname);
    WebhookUrls {
        strava: format!("{base}/strava/callback"),
        icu: format!("{base}/icu/callback"),
    }
}

/// Resolves one variable: the canonical name wins; the deprecated name is
/// the fallback (yielding a deprecation warning that names it and its
/// replacement); then the default. Panics when nothing is set and no
/// default exists.
fn resolve(
    env: &HashMap<String, String>,
    canonical: &str,
    deprecated: &str,
    default: Option<&str>,
) -> (String, Option<String>) {
    if let Some(value) = env.get(canonical) {
        return (value.clone(), None);
    }
    if let Some(value) = env.get(deprecated) {
        let warning =
            format!("deprecated environment variable {deprecated}: use {canonical} instead");
        return (value.clone(), Some(warning));
    }
    match default {
        Some(value) => (value.to_string(), None),
        None => panic!("Missing {canonical}!"),
    }
}

/// Resolves an optional variable with the same canonical/deprecated rule as
/// [`resolve`], but nothing-set yields `None` (a disabled provider,
/// ADR-0016) instead of a panic. A `None` deprecated name means the
/// variable has no legacy spelling (the canonical name only).
fn resolve_optional(
    env: &HashMap<String, String>,
    canonical: &str,
    deprecated: Option<&str>,
) -> (Option<String>, Option<String>) {
    if let Some(value) = env.get(canonical) {
        return (Some(value.clone()), None);
    }
    if let Some(deprecated) = deprecated
        && let Some(value) = env.get(deprecated)
    {
        let warning =
            format!("deprecated environment variable {deprecated}: use {canonical} instead");
        return (Some(value.clone()), Some(warning));
    }
    (None, None)
}

/// Resolves the full instance configuration from the environment, returning
/// it with the deprecation warnings for the legacy variables that supplied
/// values.
pub(crate) fn from_env(env: &HashMap<String, String>) -> (AppConfig, Vec<String>) {
    let mut warnings = Vec::new();

    let (database_url, warning) = resolve(
        env,
        "DATABASE_URL",
        "DB_URL",
        Some("postgres://localhost/tendabike"),
    );
    warnings.extend(warning);

    // The providers' credentials are optional (ADR-0016): a missing client
    // id disables the provider (a graceful freeze), it does not panic the
    // startup. The deprecated `CLIENT_ID`/`CLIENT_SECRET` fallback keeps
    // its warning.
    let (strava_client_id, warning) = resolve_optional(env, "STRAVA_CLIENT_ID", Some("CLIENT_ID"));
    warnings.extend(warning);
    let (strava_client_secret, warning) =
        resolve_optional(env, "STRAVA_CLIENT_SECRET", Some("CLIENT_SECRET"));
    warnings.extend(warning);
    // The ICU variables have no legacy spelling: canonical name only.
    let (icu_client_id, warning) = resolve_optional(env, "ICU_CLIENT_ID", None);
    warnings.extend(warning);
    let (icu_client_secret, warning) = resolve_optional(env, "ICU_CLIENT_SECRET", None);
    warnings.extend(warning);

    let hostname = env.get("HOSTNAME").map(String::as_str);
    let urls = redirect_urls(hostname);
    let webhooks = webhook_urls(hostname);

    let config = AppConfig {
        database_url,
        strava_client_id,
        strava_client_secret,
        strava_redirect_url: urls.strava,
        strava_webhook_url: webhooks.strava,
        icu_client_id,
        icu_client_secret,
        icu_redirect_url: urls.icu,
        icu_webhook_url: webhooks.icu,
        icu_webhook_secret: env.get("ICU_WEBHOOK_SECRET").cloned().unwrap_or_default(),
    };

    (config, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // --- redirect_urls --------------------------------------------------

    #[test]
    fn hostname_yields_https_redirects() {
        let urls = redirect_urls(Some("tendabike.example.com"));
        assert_eq!(urls.strava, "https://tendabike.example.com/strava/token");
        assert_eq!(urls.icu, "https://tendabike.example.com/icu/token");
    }

    #[test]
    fn localhost_hostname_yields_http_redirects() {
        let urls = redirect_urls(Some("localhost:8000"));
        assert_eq!(urls.strava, "http://localhost:8000/strava/token");
        assert_eq!(urls.icu, "http://localhost:8000/icu/token");
    }

    #[test]
    fn loopback_hostname_yields_http_redirects() {
        let urls = redirect_urls(Some("127.0.0.1:8000"));
        assert_eq!(urls.strava, "http://127.0.0.1:8000/strava/token");
        assert_eq!(urls.icu, "http://127.0.0.1:8000/icu/token");
    }

    #[test]
    fn missing_hostname_yields_local_dev_defaults() {
        let urls = redirect_urls(None);
        assert_eq!(urls.strava, "http://localhost:8000/strava/token");
        assert_eq!(urls.icu, "http://localhost:8000/icu/token");
    }

    // --- webhook_urls -----------------------------------------------------

    #[test]
    fn hostname_yields_https_webhook_urls() {
        let urls = webhook_urls(Some("tendabike.example.com"));
        assert_eq!(urls.strava, "https://tendabike.example.com/strava/callback");
        assert_eq!(urls.icu, "https://tendabike.example.com/icu/callback");
    }

    #[test]
    fn localhost_hostname_yields_http_webhook_urls() {
        let urls = webhook_urls(Some("localhost:8000"));
        assert_eq!(urls.strava, "http://localhost:8000/strava/callback");
        assert_eq!(urls.icu, "http://localhost:8000/icu/callback");
    }

    #[test]
    fn loopback_hostname_yields_http_webhook_urls() {
        let urls = webhook_urls(Some("127.0.0.1:8000"));
        assert_eq!(urls.strava, "http://127.0.0.1:8000/strava/callback");
        assert_eq!(urls.icu, "http://127.0.0.1:8000/icu/callback");
    }

    #[test]
    fn missing_hostname_yields_local_dev_webhook_defaults() {
        let urls = webhook_urls(None);
        assert_eq!(urls.strava, "http://localhost:8000/strava/callback");
        assert_eq!(urls.icu, "http://localhost:8000/icu/callback");
    }

    // --- resolve ----------------------------------------------------------

    #[test]
    fn canonical_wins_when_both_set() {
        let e = env(&[("STRAVA_CLIENT_ID", "new"), ("CLIENT_ID", "old")]);
        let (value, warning) = resolve(&e, "STRAVA_CLIENT_ID", "CLIENT_ID", None);
        assert_eq!(value, "new");
        assert!(warning.is_none());
    }

    #[test]
    fn deprecated_fallback_warns() {
        let e = env(&[("CLIENT_ID", "old")]);
        let (value, warning) = resolve(&e, "STRAVA_CLIENT_ID", "CLIENT_ID", None);
        assert_eq!(value, "old");
        let warning = warning.expect("a deprecation warning");
        assert!(warning.contains("CLIENT_ID"));
        assert!(warning.contains("STRAVA_CLIENT_ID"));
    }

    #[test]
    fn default_applies_when_nothing_set() {
        let e = HashMap::new();
        let (value, warning) = resolve(
            &e,
            "DATABASE_URL",
            "DB_URL",
            Some("postgres://localhost/tendabike"),
        );
        assert_eq!(value, "postgres://localhost/tendabike");
        assert!(warning.is_none());
    }

    #[test]
    #[should_panic]
    fn required_variable_missing_panics() {
        let e = HashMap::new();
        resolve(&e, "STRAVA_CLIENT_ID", "CLIENT_ID", None);
    }

    // --- from_env ---------------------------------------------------------

    #[test]
    fn canonical_env_resolves_without_warnings() {
        let e = env(&[
            ("DATABASE_URL", "postgres://db/tendabike"),
            ("HOSTNAME", "tendabike.example.com"),
            ("STRAVA_CLIENT_ID", "sid"),
            ("STRAVA_CLIENT_SECRET", "ssec"),
            ("ICU_CLIENT_ID", "iid"),
            ("ICU_CLIENT_SECRET", "isec"),
            ("ICU_WEBHOOK_SECRET", "hook"),
        ]);
        let (config, warnings) = from_env(&e);
        assert!(warnings.is_empty());
        assert_eq!(config.database_url, "postgres://db/tendabike");
        assert_eq!(config.strava_client_id, Some("sid".to_string()));
        assert_eq!(config.strava_client_secret, Some("ssec".to_string()));
        assert_eq!(
            config.strava_redirect_url,
            "https://tendabike.example.com/strava/token"
        );
        assert_eq!(
            config.strava_webhook_url,
            "https://tendabike.example.com/strava/callback"
        );
        assert_eq!(config.icu_client_id, Some("iid".to_string()));
        assert_eq!(config.icu_client_secret, Some("isec".to_string()));
        assert_eq!(
            config.icu_redirect_url,
            "https://tendabike.example.com/icu/token"
        );
        assert_eq!(
            config.icu_webhook_url,
            "https://tendabike.example.com/icu/callback"
        );
        assert_eq!(config.icu_webhook_secret, "hook");
    }

    #[test]
    fn legacy_env_resolves_with_warnings() {
        let e = env(&[
            ("DB_URL", "postgres://db/tendabike"),
            ("CLIENT_ID", "sid"),
            ("CLIENT_SECRET", "ssec"),
            ("ICU_CLIENT_ID", "iid"),
            ("ICU_CLIENT_SECRET", "isec"),
        ]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.database_url, "postgres://db/tendabike");
        assert_eq!(config.strava_client_id, Some("sid".to_string()));
        assert_eq!(config.strava_client_secret, Some("ssec".to_string()));
        assert_eq!(
            config.strava_redirect_url,
            "http://localhost:8000/strava/token"
        );
        assert_eq!(
            config.strava_webhook_url,
            "http://localhost:8000/strava/callback"
        );
        assert_eq!(config.icu_redirect_url, "http://localhost:8000/icu/token");
        assert_eq!(config.icu_webhook_url, "http://localhost:8000/icu/callback");
        assert_eq!(warnings.len(), 3);
        for (legacy, replacement) in [
            ("DB_URL", "DATABASE_URL"),
            ("CLIENT_ID", "STRAVA_CLIENT_ID"),
            ("CLIENT_SECRET", "STRAVA_CLIENT_SECRET"),
        ] {
            let hit = warnings
                .iter()
                .find(|w| w.contains(legacy))
                .unwrap_or_else(|| panic!("no warning naming {legacy}"));
            assert!(
                hit.contains(replacement),
                "warning for {legacy} must name {replacement}: {hit}"
            );
        }
    }

    // --- subset providers (ADR-0016) --------------------------------------

    /// A missing Strava client id (canonical and deprecated) disables the
    /// provider — it does not panic the startup.
    #[test]
    fn missing_strava_client_id_disables_strava() {
        let e = env(&[("ICU_CLIENT_ID", "iid"), ("ICU_CLIENT_SECRET", "isec")]);
        let (config, warnings) = from_env(&e);
        assert!(config.strava_client_id.is_none());
        assert!(config.strava_client_secret.is_none());
        assert_eq!(config.icu_client_id, Some("iid".to_string()));
        assert!(warnings.is_empty());
    }

    /// A missing intervals.icu client id disables the provider — it does
    /// not panic the startup.
    #[test]
    fn missing_icu_client_id_disables_icu() {
        let e = env(&[
            ("STRAVA_CLIENT_ID", "sid"),
            ("STRAVA_CLIENT_SECRET", "ssec"),
        ]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.strava_client_id, Some("sid".to_string()));
        assert!(config.icu_client_id.is_none());
        assert!(config.icu_client_secret.is_none());
        assert!(warnings.is_empty());
    }

    /// Both client ids missing: the instance runs with no providers (the
    /// frontend shows the failure page) — the startup does not panic.
    #[test]
    fn no_client_id_disables_every_provider() {
        let e = HashMap::new();
        let (config, warnings) = from_env(&e);
        assert!(config.strava_client_id.is_none());
        assert!(config.icu_client_id.is_none());
        assert!(warnings.is_empty());
    }

    /// The deprecated `CLIENT_ID`/`CLIENT_SECRET` fallback still enables
    /// Strava and still warns: only a *missing* client id (canonical and
    /// fallback both absent) disables the provider.
    #[test]
    fn deprecated_fallback_still_enables_and_warns() {
        let e = env(&[("CLIENT_ID", "sid"), ("CLIENT_SECRET", "ssec")]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.strava_client_id, Some("sid".to_string()));
        assert_eq!(config.strava_client_secret, Some("ssec".to_string()));
        assert!(config.icu_client_id.is_none());
        assert_eq!(warnings.len(), 2);
        for (legacy, replacement) in [
            ("CLIENT_ID", "STRAVA_CLIENT_ID"),
            ("CLIENT_SECRET", "STRAVA_CLIENT_SECRET"),
        ] {
            let hit = warnings
                .iter()
                .find(|w| w.contains(legacy))
                .unwrap_or_else(|| panic!("no warning naming {legacy}"));
            assert!(
                hit.contains(replacement),
                "warning for {legacy} must name {replacement}: {hit}"
            );
        }
    }

    #[test]
    fn removed_redirect_vars_are_ignored() {
        let e = env(&[
            ("STRAVA_CLIENT_ID", "sid"),
            ("STRAVA_CLIENT_SECRET", "ssec"),
            ("ICU_CLIENT_ID", "iid"),
            ("ICU_CLIENT_SECRET", "isec"),
            ("REDIRECT_URL", "http://example.com/strava/token"),
            ("ICU_REDIRECT_URL", "http://example.com/icu/token"),
        ]);
        let (config, warnings) = from_env(&e);
        assert_eq!(
            config.strava_redirect_url,
            "http://localhost:8000/strava/token"
        );
        assert_eq!(config.icu_redirect_url, "http://localhost:8000/icu/token");
        assert!(warnings.is_empty());
    }
}
