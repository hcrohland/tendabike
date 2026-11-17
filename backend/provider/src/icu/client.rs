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
    AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, ExtraTokenFields, RedirectUrl,
    StandardTokenResponse, TokenUrl, basic::BasicTokenType,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use super::IcuId;
use crate::client::ProviderClient;
use crate::config;

pub use crate::client::HTTP_CLIENT;

/// The icu athlete info carried in the token response's extra fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IcuAthleteInfo {
    pub id: IcuId,
    pub name: String,
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
> = ProviderClient<
    IcuTokenResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

/// The shared icu oauth client (built once from the provider config).
static ICU_CLIENT: OnceLock<IcuClient> = OnceLock::new();

/// The initialized icu oauth client (panics if the config is not set).
/// Only called for an enabled provider (ADR-0016): a disabled provider's
/// ops are guarded before they reach the client, so the credentials are
/// present.
pub(crate) fn icu_client() -> &'static IcuClient {
    ICU_CLIENT.get_or_init(|| {
        let config = config::get();
        let client_id = ClientId::new(
            config
                .icu_client_id
                .clone()
                .expect("icu provider disabled — the client must not be built"),
        );
        let client_secret = ClientSecret::new(
            config
                .icu_client_secret
                .clone()
                .expect("icu provider disabled — the client must not be built"),
        );
        let redirect_url = RedirectUrl::new(config.icu_redirect_url.clone()).unwrap();

        let auth_url = AuthUrl::new("https://intervals.icu/oauth/authorize".to_string()).unwrap();
        let token_url = TokenUrl::new("https://intervals.icu/api/oauth/token".to_string()).unwrap();

        IcuClient::new(client_id)
            .set_client_secret(client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_auth_type(oauth2::AuthType::RequestBody)
            .set_redirect_uri(redirect_url)
    })
}

/// The scope string the login exchange requests.
pub const SCOPES: &str = "ACTIVITY:READ,SETTINGS:READ";
