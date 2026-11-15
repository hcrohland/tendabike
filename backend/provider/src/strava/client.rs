/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! The Strava API client and its wire types: the `TokenCache`'s
//! token-refresh machinery needs the oauth client, so the client lives with
//! the Strava driver, not the presentation layer. The web layer's login
//! exchange uses the same client; the wire-shape *parsing* of a token response
//! stays in the web layer, beside the exchange it feeds.

use oauth2::{
    AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, ExtraTokenFields, RedirectUrl,
    RevocationUrl, StandardTokenResponse, TokenUrl, basic::BasicTokenType,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::client::ProviderClient;
use crate::config;

pub use crate::client::HTTP_CLIENT;

/// The Strava athlete info carried in the token response's extra fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StravaAthleteInfo {
    pub id: super::StravaId,
    pub firstname: String,
    pub lastname: String,
    #[serde(rename = "profile_medium")]
    pub avatar: Option<String>,
    // profile: String,
    // bio: String,
    // city: String,
    // state: String,
    // country: String,
    // sex: String,
    // premium: bool,
    // summit: bool,
    // created_at: String,
    // updated_at: String,
    // badge_type_id: i32,
    // weight: f32,
    #[serde(flatten)]
    other: serde_json::Value,
}

/// The token response's extra fields: the athlete info the login exchange
/// parses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StravaExtraTokenFields {
    pub athlete: Option<StravaAthleteInfo>,
}

impl ExtraTokenFields for StravaExtraTokenFields {}

/// The Strava token response type (the standard response plus the athlete
/// extra fields).
pub type StravaTokenResponse = StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>;

/// The Strava oauth2 client type.
pub type StravaClient<
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointSet,
    HasTokenUrl = EndpointSet,
> = ProviderClient<
    StravaTokenResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

/// The shared Strava oauth client (built once from the provider config).
static STRAVACLIENT: OnceLock<StravaClient> = OnceLock::new();

/// The initialized Strava oauth client (panics if the config is not set).
pub(crate) fn strava_client() -> &'static StravaClient {
    STRAVACLIENT.get_or_init(|| {
        let config = config::get();
        let client_id = ClientId::new(config.strava_client_id.clone());
        let client_secret = ClientSecret::new(config.strava_client_secret.clone());
        let redirect_url = RedirectUrl::new(config.strava_redirect_url.clone()).unwrap();

        let auth_url = AuthUrl::new("https://www.strava.com/oauth/authorize".to_string()).unwrap();
        let token_url = TokenUrl::new("https://www.strava.com/oauth/token".to_string()).unwrap();
        let revocation_url =
            RevocationUrl::new("https://www.strava.com/oauth/deauthorize".to_string()).unwrap();

        StravaClient::new(client_id)
            .set_client_secret(client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_auth_type(oauth2::AuthType::RequestBody)
            .set_redirect_uri(redirect_url)
            .set_revocation_url(revocation_url)
    })
}

/// The scope string the login exchange requests.
pub const SCOPES: &str = "read,activity:read_all,profile:read_all";
