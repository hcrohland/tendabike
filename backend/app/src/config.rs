//! Startup configuration: resolves the process's environment into the
//! concrete config the crates consume.
//!
//! The public host (`HOSTNAME`) is the single source for both providers'
//! redirect URLs. `STRAVA_*` and `DATABASE_URL` are the canonical variable
//! names; `CLIENT_ID`, `CLIENT_SECRET`, and `DB_URL` remain as deprecated
//! fallbacks — a warning is produced for each one that actually supplies a
//! value.

use std::collections::HashMap;

/// The resolved instance configuration: the database URL, both providers'
/// credentials and redirect URLs, and the ICU webhook secret.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub strava_client_id: String,
    pub strava_client_secret: String,
    pub strava_redirect_url: String,
    pub icu_client_id: String,
    pub icu_client_secret: String,
    pub icu_redirect_url: String,
    pub icu_webhook_secret: String,
}

/// The provider redirect URLs derived from the instance's public host:
/// `https://<HOSTNAME>/strava/token` and `https://<HOSTNAME>/icu/token`,
/// with `http` when the host part is `localhost` or `127.0.0.1`. An unset
/// `HOSTNAME` yields the local-dev defaults (`http://localhost:8000/…`).
pub fn redirect_urls(hostname: Option<&str>) -> (String, String) {
    let host = hostname.unwrap_or("localhost:8000");
    let host_part = host.split(':').next().unwrap_or(host);
    let scheme = if matches!(host_part, "localhost" | "127.0.0.1") {
        "http"
    } else {
        "https"
    };
    (
        format!("{scheme}://{host}/strava/token"),
        format!("{scheme}://{host}/icu/token"),
    )
}

/// Resolves one variable: the canonical name wins; the deprecated name is
/// the fallback (yielding a deprecation warning that names it and its
/// replacement); then the default. Panics when nothing is set and no
/// default exists.
pub fn resolve(
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

/// Resolves the full instance configuration from the environment, returning
/// it with the deprecation warnings for the legacy variables that supplied
/// values.
pub fn from_env(env: &HashMap<String, String>) -> (AppConfig, Vec<String>) {
    let mut warnings = Vec::new();

    let (database_url, warning) = resolve(
        env,
        "DATABASE_URL",
        "DB_URL",
        Some("postgres://localhost/tendabike"),
    );
    if let Some(w) = warning {
        warnings.push(w);
    }

    let (strava_client_id, warning) = resolve(env, "STRAVA_CLIENT_ID", "CLIENT_ID", None);
    if let Some(w) = warning {
        warnings.push(w);
    }
    let (strava_client_secret, warning) =
        resolve(env, "STRAVA_CLIENT_SECRET", "CLIENT_SECRET", None);
    if let Some(w) = warning {
        warnings.push(w);
    }

    let (strava_redirect_url, icu_redirect_url) =
        redirect_urls(env.get("HOSTNAME").map(String::as_str));

    let config = AppConfig {
        database_url,
        strava_client_id,
        strava_client_secret,
        strava_redirect_url,
        icu_client_id: env
            .get("ICU_CLIENT_ID")
            .cloned()
            .expect("Missing ICU_CLIENT_ID!"),
        icu_client_secret: env
            .get("ICU_CLIENT_SECRET")
            .cloned()
            .expect("Missing ICU_CLIENT_SECRET!"),
        icu_redirect_url,
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
        let (strava, icu) = redirect_urls(Some("tendabike.example.com"));
        assert_eq!(strava, "https://tendabike.example.com/strava/token");
        assert_eq!(icu, "https://tendabike.example.com/icu/token");
    }

    #[test]
    fn localhost_hostname_yields_http_redirects() {
        let (strava, icu) = redirect_urls(Some("localhost:8000"));
        assert_eq!(strava, "http://localhost:8000/strava/token");
        assert_eq!(icu, "http://localhost:8000/icu/token");
    }

    #[test]
    fn loopback_hostname_yields_http_redirects() {
        let (strava, icu) = redirect_urls(Some("127.0.0.1:8000"));
        assert_eq!(strava, "http://127.0.0.1:8000/strava/token");
        assert_eq!(icu, "http://127.0.0.1:8000/icu/token");
    }

    #[test]
    fn missing_hostname_yields_local_dev_defaults() {
        let (strava, icu) = redirect_urls(None);
        assert_eq!(strava, "http://localhost:8000/strava/token");
        assert_eq!(icu, "http://localhost:8000/icu/token");
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
        assert_eq!(config.strava_client_id, "sid");
        assert_eq!(config.strava_client_secret, "ssec");
        assert_eq!(
            config.strava_redirect_url,
            "https://tendabike.example.com/strava/token"
        );
        assert_eq!(config.icu_client_id, "iid");
        assert_eq!(config.icu_client_secret, "isec");
        assert_eq!(
            config.icu_redirect_url,
            "https://tendabike.example.com/icu/token"
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
        assert_eq!(config.strava_client_id, "sid");
        assert_eq!(config.strava_client_secret, "ssec");
        assert_eq!(
            config.strava_redirect_url,
            "http://localhost:8000/strava/token"
        );
        assert_eq!(config.icu_redirect_url, "http://localhost:8000/icu/token");
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
