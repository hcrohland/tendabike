//! The provider's redirect and webhook callback URLs, composed from the
//! instance's public domain.
//!
//! The domain (`DOMAINNAME`) is the single source for both providers'
//! redirect and webhook callback URLs: `https://<DOMAINNAME>/strava/token`,
//! `…/icu/token` (redirects) and `…/strava/callback`, `…/icu/callback`
//! (webhooks). `http` is used when the host part is `localhost` or
//! `127.0.0.1`.

/// The base URL (`<scheme>://<host>`) derived from the instance's public
/// domain: `http` when the host part is `localhost` or `127.0.0.1`, `https`
/// otherwise.
fn base_url(domainname: &str) -> String {
    let host_part = domainname.split(':').next().unwrap_or(domainname);
    let scheme = if matches!(host_part, "localhost" | "127.0.0.1") {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{domainname}")
}

/// The provider redirect URLs: `https://<DOMAINNAME>/strava/token` and
/// `https://<DOMAINNAME>/icu/token`, with `http` when the host part is
/// `localhost` or `127.0.0.1`.
pub(crate) struct RedirectUrls {
    pub(crate) strava: String,
    pub(crate) icu: String,
}

pub(crate) fn redirect_urls(domainname: &str) -> RedirectUrls {
    let base = base_url(domainname);
    RedirectUrls {
        strava: format!("{base}/strava/token"),
        icu: format!("{base}/icu/token"),
    }
}

/// The provider webhook callback URLs: `https://<DOMAINNAME>/strava/callback`
/// and `https://<DOMAINNAME>/icu/callback`, with `http` when the host part is
/// `localhost` or `127.0.0.1`.
pub(crate) struct WebhookUrls {
    pub(crate) strava: String,
    pub(crate) icu: String,
}

pub(crate) fn webhook_urls(domainname: &str) -> WebhookUrls {
    let base = base_url(domainname);
    WebhookUrls {
        strava: format!("{base}/strava/callback"),
        icu: format!("{base}/icu/callback"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- redirect_urls --------------------------------------------------

    #[test]
    fn domainname_yields_https_redirects() {
        let urls = redirect_urls("tendabike.example.com");
        assert_eq!(urls.strava, "https://tendabike.example.com/strava/token");
        assert_eq!(urls.icu, "https://tendabike.example.com/icu/token");
    }

    #[test]
    fn localhost_domainname_yields_http_redirects() {
        let urls = redirect_urls("localhost:8000");
        assert_eq!(urls.strava, "http://localhost:8000/strava/token");
        assert_eq!(urls.icu, "http://localhost:8000/icu/token");
    }

    #[test]
    fn loopback_domainname_yields_http_redirects() {
        let urls = redirect_urls("127.0.0.1:8000");
        assert_eq!(urls.strava, "http://127.0.0.1:8000/strava/token");
        assert_eq!(urls.icu, "http://127.0.0.1:8000/icu/token");
    }

    // --- webhook_urls -----------------------------------------------------

    #[test]
    fn domainname_yields_https_webhook_urls() {
        let urls = webhook_urls("tendabike.example.com");
        assert_eq!(urls.strava, "https://tendabike.example.com/strava/callback");
        assert_eq!(urls.icu, "https://tendabike.example.com/icu/callback");
    }

    #[test]
    fn localhost_domainname_yields_http_webhook_urls() {
        let urls = webhook_urls("localhost:8000");
        assert_eq!(urls.strava, "http://localhost:8000/strava/callback");
        assert_eq!(urls.icu, "http://localhost:8000/icu/callback");
    }

    #[test]
    fn loopback_domainname_yields_http_webhook_urls() {
        let urls = webhook_urls("127.0.0.1:8000");
        assert_eq!(urls.strava, "http://127.0.0.1:8000/strava/callback");
        assert_eq!(urls.icu, "http://127.0.0.1:8000/icu/callback");
    }
}
