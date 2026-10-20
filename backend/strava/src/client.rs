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

//! The Strava API client and its wire types (moved out of the web layer's
//! oauth module, ADR-0007, executable spec #558 §6.2): the concrete session
//! (`tb_strava::StravaUserSession`) needs the oauth client for its
//! token-refresh machinery, so the client lives with the Strava driver, not
//! the presentation layer. The web layer's login exchange uses the same
//! client; the wire-shape *parsing* of a token response stays in the web
//! layer, beside the exchange it feeds.

use oauth2::{
    AuthUrl, Client, ClientId, ClientSecret, EndpointNotSet, EndpointSet, ExtraTokenFields,
    RedirectUrl, RevocationUrl, StandardRevocableToken, StandardTokenResponse, TokenUrl,
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::LazyLock};

/// The Strava athlete info carried in the token response's extra fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StravaAthleteInfo {
    pub id: crate::StravaId,
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
> = Client<
    BasicErrorResponse,
    StravaTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

/// The shared Strava oauth client (built once from the process environment).
pub static STRAVACLIENT: LazyLock<StravaClient> = LazyLock::new(strava_oauth_client);

/// The shared reqwest client for the Strava API (no redirects: following
/// them opens the client up to SSRF vulnerabilities).
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(http_client);

pub fn strava_oauth_client() -> StravaClient {
    // Environment variables (* = required):
    // *"CLIENT_ID"     "REPLACE_ME";
    // *"CLIENT_SECRET" "REPLACE_ME";
    //  "REDIRECT_URL"  "http://127.0.0.1:3000/auth/authorized";
    //  "AUTH_URL"      "https://www.strava.com/oauth/authorize";
    //  "TOKEN_URL"     "https://www.strava.com/oauth/token";

    let client_id = ClientId::new(env::var("CLIENT_ID").expect("Missing CLIENT_ID!"));
    let client_secret =
        ClientSecret::new(env::var("CLIENT_SECRET").expect("Missing CLIENT_SECRET!"));

    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8000/strava/token".to_string());
    let redirect_url = RedirectUrl::new(redirect_url).unwrap();

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
}

fn http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build")
}

/// The scope string the login exchange requests.
pub const SCOPES: &str = "read,activity:read_all,profile:read_all";
