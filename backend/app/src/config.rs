//! Startup configuration: resolves the process's environment into the
//! concrete config the crates consume.
//!
//! Every variable is resolved here (the composition root reads none directly):
//! the database URL and the web layer's config — a `tb_axum::AxumConfig`
//! carrying the public domain (`DOMAINNAME`, the single source for both
//! providers' redirect and webhook callback URLs), the provider credentials,
//! the CSRF secret, the static files path, and the bind address.
//! `DOMAINNAME`, `STRAVA_*`, and `DATABASE_URL` are the canonical variable
//! names; `HOSTNAME`, `CLIENT_ID`, `CLIENT_SECRET`, and `DB_URL` remain as
//! deprecated fallbacks — a warning is produced for each one that actually
//! supplies a value.

use std::collections::HashMap;

use tb_axum::AxumConfig;

/// The resolved instance configuration: the database URL and the web layer's
/// config (a `tb_axum::AxumConfig` — the public domain, the provider
/// credentials, the CSRF secret, the static files path, and the bind address).
#[derive(Debug, Clone)]
pub(crate) struct AppConfig {
    pub(crate) database_url: String,
    pub(crate) axum: AxumConfig,
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

    // The public domain defaults to the local-dev host: `DOMAINNAME` is the
    // canonical name, `HOSTNAME` the deprecated fallback.
    let (domainname, warning) = resolve(env, "DOMAINNAME", "HOSTNAME", Some("localhost:8000"));
    warnings.extend(warning);

    // The web layer's config: the public domain, the provider credentials,
    // the CSRF secret, the static files path, and the bind address.
    let axum = AxumConfig {
        csrf_secret: env.get("CSRF_SECRET").cloned(),
        static_www: env.get("STATIC_WWW").cloned().unwrap_or_else(|| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../frontend/build").to_string()
        }),
        bind_addr: env
            .get("BIND_ADDR")
            .cloned()
            .unwrap_or_else(|| "127.0.0.1:8000".to_string()),
        domainname,
        strava_client_id,
        strava_client_secret,
        icu_client_id,
        icu_client_secret,
        icu_webhook_secret: env.get("ICU_WEBHOOK_SECRET").cloned().unwrap_or_default(),
    };

    let config = AppConfig { database_url, axum };

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
            ("DOMAINNAME", "tendabike.example.com"),
            ("STRAVA_CLIENT_ID", "sid"),
            ("STRAVA_CLIENT_SECRET", "ssec"),
            ("ICU_CLIENT_ID", "iid"),
            ("ICU_CLIENT_SECRET", "isec"),
            ("ICU_WEBHOOK_SECRET", "hook"),
        ]);
        let (config, warnings) = from_env(&e);
        assert!(warnings.is_empty());
        assert_eq!(config.database_url, "postgres://db/tendabike");
        assert_eq!(config.axum.domainname, "tendabike.example.com");
        assert_eq!(config.axum.strava_client_id, Some("sid".to_string()));
        assert_eq!(config.axum.strava_client_secret, Some("ssec".to_string()));
        assert_eq!(config.axum.icu_client_id, Some("iid".to_string()));
        assert_eq!(config.axum.icu_client_secret, Some("isec".to_string()));
        assert_eq!(config.axum.icu_webhook_secret, "hook");
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
        // No `DOMAINNAME`/`HOSTNAME` set: the domainname is the local-dev
        // default.
        assert_eq!(config.axum.domainname, "localhost:8000");
        assert_eq!(config.axum.strava_client_id, Some("sid".to_string()));
        assert_eq!(config.axum.strava_client_secret, Some("ssec".to_string()));
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

    // --- domainname (DOMAINNAME canonical, HOSTNAME deprecated) -----------

    /// `DOMAINNAME` is the canonical variable: it resolves the domainname and
    /// emits no deprecation warning.
    #[test]
    fn domainname_canonical_resolves_without_warning() {
        let e = env(&[("DOMAINNAME", "tendabike.example.com")]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.axum.domainname, "tendabike.example.com");
        assert!(warnings.is_empty());
    }

    /// Only `HOSTNAME` set (no `DOMAINNAME`): the value is used and a
    /// deprecation warning names both `HOSTNAME` and `DOMAINNAME`.
    #[test]
    fn hostname_deprecated_fallback_resolves_with_warning() {
        let e = env(&[("HOSTNAME", "tendabike.example.com")]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.axum.domainname, "tendabike.example.com");
        let hit = warnings
            .iter()
            .find(|w| w.contains("HOSTNAME"))
            .expect("a deprecation warning naming HOSTNAME");
        assert!(
            hit.contains("DOMAINNAME"),
            "the HOSTNAME warning must name DOMAINNAME: {hit}"
        );
    }

    /// `DOMAINNAME` wins when both are set (the canonical name takes
    /// precedence over the deprecated fallback), and no warning is emitted.
    #[test]
    fn domainname_wins_when_both_set() {
        let e = env(&[
            ("DOMAINNAME", "canonical.example.com"),
            ("HOSTNAME", "deprecated.example.com"),
        ]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.axum.domainname, "canonical.example.com");
        assert!(warnings.is_empty());
    }

    /// Neither `DOMAINNAME` nor `HOSTNAME` set: the domainname is the
    /// local-dev default and no deprecation warning is emitted.
    #[test]
    fn neither_domainname_nor_hostname_yields_local_dev_default() {
        let e = HashMap::new();
        let (config, warnings) = from_env(&e);
        assert_eq!(config.axum.domainname, "localhost:8000");
        assert!(warnings.is_empty());
    }

    // --- subset providers (ADR-0016) --------------------------------------

    /// A missing Strava client id (canonical and deprecated) disables the
    /// provider — it does not panic the startup.
    #[test]
    fn missing_strava_client_id_disables_strava() {
        let e = env(&[("ICU_CLIENT_ID", "iid"), ("ICU_CLIENT_SECRET", "isec")]);
        let (config, warnings) = from_env(&e);
        assert!(config.axum.strava_client_id.is_none());
        assert!(config.axum.strava_client_secret.is_none());
        assert_eq!(config.axum.icu_client_id, Some("iid".to_string()));
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
        assert_eq!(config.axum.strava_client_id, Some("sid".to_string()));
        assert!(config.axum.icu_client_id.is_none());
        assert!(config.axum.icu_client_secret.is_none());
        assert!(warnings.is_empty());
    }

    /// Both client ids missing: the instance runs with no providers (the
    /// frontend shows the failure page) — the startup does not panic.
    #[test]
    fn no_client_id_disables_every_provider() {
        let e = HashMap::new();
        let (config, warnings) = from_env(&e);
        assert!(config.axum.strava_client_id.is_none());
        assert!(config.axum.icu_client_id.is_none());
        assert!(warnings.is_empty());
    }

    /// The deprecated `CLIENT_ID`/`CLIENT_SECRET` fallback still enables
    /// Strava and still warns: only a *missing* client id (canonical and
    /// fallback both absent) disables the provider.
    #[test]
    fn deprecated_fallback_still_enables_and_warns() {
        let e = env(&[("CLIENT_ID", "sid"), ("CLIENT_SECRET", "ssec")]);
        let (config, warnings) = from_env(&e);
        assert_eq!(config.axum.strava_client_id, Some("sid".to_string()));
        assert_eq!(config.axum.strava_client_secret, Some("ssec".to_string()));
        assert!(config.axum.icu_client_id.is_none());
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
}
