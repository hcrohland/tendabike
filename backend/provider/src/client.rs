//! The shared client scaffolding both providers use (spec #838): the
//! no-redirect `reqwest` client — built once and shared by every provider
//! call — the oauth2 client type alias, parameterized by the token response
//! type, and the [`OauthClient`] helper trait (the full oauth2 client shape
//! named once, spec #1062). The providers keep their own client *values*
//! (built from their own env vars and endpoints) and their token-response
//! wire types.

use oauth2::{
    Client, EndpointNotSet, EndpointSet, EndpointState, RefreshToken, RefreshTokenRequest,
    StandardRevocableToken, TokenResponse,
    basic::{BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse},
    reqwest,
};
use std::sync::LazyLock;

/// The shared `reqwest` client for the provider APIs (no redirects:
/// following them opens the client up to SSRF vulnerabilities).
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(http_client);

/// Builds the shared no-redirect `reqwest` client.
fn http_client() -> reqwest::Client {
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

/// The full oauth2 client shape (spec #1062): the common provider client's
/// exact endpoint-marker set — the auth and token URLs set, the device-auth
/// and introspection URLs unset, the revocation URL free (a provider with a
/// revocation endpoint sets it — Strava — and one without leaves it unset —
/// intervals.icu).
///
/// A helper trait the driver trait's `Client` associated type is bounded by:
/// the common provider client is a type alias ([`ProviderClient`]), which
/// cannot appear in a trait bound, so the shape is named by the trait.
/// Blanket-implemented for every provider client (every
/// [`ProviderClient`] over the marker set above), so a new provider's client
/// is checked against the shape at the trait bound, not at the exchange call
/// site. The trait carries the one method the kit's refresh exchange drives
/// (the standard oauth2 refresh-token request), so the exchange is generic
/// over the bound, not over the concrete client type. `Sync` and the
/// token response's `Send` are part of the shape: the driver trait's
/// `refresh` default awaits the exchange across a thread (the `async_trait`
/// `Send` bound), holding the shared client and the response type.
pub trait OauthClient: Sync {
    /// The client's token response type (the wire shape the exchange parses).
    type TokenResponse: TokenResponse + Send;

    /// The standard oauth2 refresh-token request (the kit's exchange drives
    /// it over the shared no-redirect HTTP client).
    fn exchange_refresh_token<'a>(
        &'a self,
        refresh_token: &'a RefreshToken,
    ) -> RefreshTokenRequest<'a, BasicErrorResponse, Self::TokenResponse>;
}

impl<TR, R> OauthClient
    for ProviderClient<TR, EndpointSet, EndpointNotSet, EndpointNotSet, R, EndpointSet>
where
    TR: TokenResponse + Send + Sync,
    R: EndpointState + Send + Sync,
{
    type TokenResponse = TR;

    fn exchange_refresh_token<'a>(
        &'a self,
        refresh_token: &'a RefreshToken,
    ) -> RefreshTokenRequest<'a, BasicErrorResponse, TR> {
        self.exchange_refresh_token(refresh_token)
    }
}
