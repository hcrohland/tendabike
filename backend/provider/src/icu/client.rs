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

//! The intervals.icu API client and its wire types: the web layer's login
//! exchange uses the oauth client; the wire-shape *parsing* of a token
//! response stays in the web layer, beside the exchange it feeds (mirroring
//! the Strava driver's split).

use oauth2::{
    AuthUrl, Client, ClientId, ClientSecret, EndpointNotSet, EndpointSet, ExtraTokenFields,
    RedirectUrl, StandardRevocableToken, StandardTokenResponse, TokenUrl,
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::LazyLock};

use super::IcuId;

/// The icu athlete info carried in the token response's extra fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IcuAthleteInfo {
    pub id: IcuId,
    pub firstname: String,
    pub lastname: String,
    #[serde(rename = "profile_medium")]
    pub avatar: Option<String>,
    #[serde(flatten)]
    other: serde_json::Value,
}

/// The token response's extra fields: the athlete info the login exchange
/// parses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IcuExtraTokenFields {
    pub athlete: Option<IcuAthleteInfo>,
}

impl ExtraTokenFields for IcuExtraTokenFields {}

/// The icu token response type (the standard response plus the athlete extra
/// fields).
pub type IcuTokenResponse = StandardTokenResponse<IcuExtraTokenFields, BasicTokenType>;

/// The icu oauth2 client type. intervals.icu exposes no token revocation
/// endpoint, so the revocation URL is `EndpointNotSet` (the `icu_deauthorize`
/// op is a local-only disable — spec #763).
pub type IcuClient<
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointSet,
> = Client<
    BasicErrorResponse,
    IcuTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

/// The shared icu oauth client (built once from the process environment).
pub static ICU_CLIENT: LazyLock<IcuClient> = LazyLock::new(icu_oauth_client);

/// The shared reqwest client for the icu API (no redirects: following
/// them opens the client up to SSRF vulnerabilities).
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(http_client);

fn icu_oauth_client() -> IcuClient {
    // Environment variables (* = required):
    // *"ICU_CLIENT_ID"     "REPLACE_ME";
    // *"ICU_CLIENT_SECRET" "REPLACE_ME";
    //  "ICU_REDIRECT_URL"  "http://127.0.0.1:3000/icu/token";
    //  "ICU_AUTH_URL"      "https://intervals.icu/oauth/authorize";
    //  "ICU_TOKEN_URL"     "https://intervals.icu/api/v1/oauth/token";

    let client_id = ClientId::new(env::var("ICU_CLIENT_ID").expect("Missing ICU_CLIENT_ID!"));
    let client_secret =
        ClientSecret::new(env::var("ICU_CLIENT_SECRET").expect("Missing ICU_CLIENT_SECRET!"));

    let redirect_url = env::var("ICU_REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8000/icu/token".to_string());
    let redirect_url = RedirectUrl::new(redirect_url).unwrap();

    let auth_url = AuthUrl::new("https://intervals.icu/oauth/authorize".to_string()).unwrap();
    let token_url = TokenUrl::new("https://intervals.icu/api/v1/oauth/token".to_string()).unwrap();

    IcuClient::new(client_id)
        .set_client_secret(client_secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_auth_type(oauth2::AuthType::RequestBody)
        .set_redirect_uri(redirect_url)
}

fn http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build")
}

/// The scope string the login exchange requests.
pub const SCOPES: &str = "ACTIVITY:READ";
