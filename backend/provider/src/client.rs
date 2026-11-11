//! The shared client scaffolding both providers used to duplicate (spec
//! #838): the no-redirect `reqwest` client — built once and shared by every
//! provider call — and the oauth2 client type alias, parameterized by the
//! token response type. The providers keep their own client *values* (built
//! from their own env vars and endpoints) and their token-response wire
//! types.

use oauth2::{
    Client, EndpointNotSet, EndpointSet, StandardRevocableToken,
    basic::{BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse},
    reqwest,
};
use std::sync::LazyLock;

/// The shared `reqwest` client for the provider APIs (no redirects:
/// following them opens the client up to SSRF vulnerabilities).
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(http_client);

/// Builds the shared no-redirect `reqwest` client.
pub fn http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build")
}

/// The providers' common oauth2 client type, parameterized by the token
/// response type (whose extra fields are provider-specific) and the
/// endpoint-state markers. Providers build from the all-unset client
/// (`::new`, which unifies the markers) and set the URLs they have — Strava
/// sets a revocation URL, intervals.icu does not (spec #763). The markers
/// stay generic parameters: `::new` is only defined on the all-unset
/// client, and it is resolved through the alias's un-instantiated markers.
pub type ProviderClient<
    TokenResponse,
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointSet,
> = Client<
    BasicErrorResponse,
    TokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;
