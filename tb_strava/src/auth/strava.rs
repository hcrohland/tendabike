use crate::*;
use rocket::request::Request;
use rocket::response::Redirect;
pub use rocket_oauth2::hyper_sync_rustls_adapter::HyperSyncRustlsAdapter;
use rocket_oauth2::{OAuth2, OAuthConfig, TokenResponse};

const PROVIDER: rocket_oauth2::StaticProvider = rocket_oauth2::StaticProvider {
    auth_uri: std::borrow::Cow::Borrowed("https://www.strava.com/oauth/authorize"),
    token_uri: std::borrow::Cow::Borrowed("https://www.strava.com/oauth/token"),
};

pub(crate) const API: &str = "https://www.strava.com/api/v3";

lazy_static! {
    static ref CLIENT_ID: String = std::env::var("STRAVA_ID").expect("Couldn't read var STRAVA_ID");
    static ref CLIENT_SECRET: String =
        std::env::var("STRAVA_SECRET").expect("Couldn't read var STRAVA_SECRET");
    static ref CLIENT_CALLBACK: String =
        std::env::var("STRAVA_CALLBACK").expect("Couldn't read var STRAVA_SECRET");
}

// We need a struct Strava to identify its type
// which is needed to retrieve the request guard
#[derive(Debug)]
pub struct Strava;

impl rocket_oauth2::Callback for Strava {
    type Responder = TbResult<Redirect>;
    fn callback(&self, request: &Request, token: TokenResponse) -> TbResult<Redirect> {
        info!("Strava got scope {:?}", token.scope());
        let athlete = token
            .as_value()
            .get("athlete")
            .ok_or(OAuthError::Authorize("token did not include athlete"))?;

        auth::DbUser::retrieve(request, athlete)?.store(request, token)?;
        Ok(Redirect::to("/"))
    }
}

pub type OAuth = OAuth2<Strava>;

pub fn fairing() -> impl rocket::fairing::Fairing {
    let config = OAuthConfig::new(
        PROVIDER,
        CLIENT_ID.to_string(),
        CLIENT_SECRET.to_string(),
        CLIENT_CALLBACK.to_string(),
    );

    // Strava uses "," instead of the standard Space as a delimter for scopes :-(
    OAuth2::custom(
        HyperSyncRustlsAdapter,
        Strava {},
        config,
        "/token",
        Some((
            "/login",
            vec!["activity:read_all,profile:read_all".to_string()],
        )),
    )
}
