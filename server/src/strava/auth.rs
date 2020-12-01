use rocket::http::*;
use rocket::request::{self, FromRequest, Request, State};
use rocket::response::Redirect;
use rocket::*;
use super::*;
use crate::token;
use rocket::Config;

pub use rocket_oauth2::HyperSyncRustlsAdapter;
use rocket_oauth2::{OAuth2, TokenResponse, OAuthConfig};

use serde_json::Value;

use diesel::prelude::*;
use diesel::{self, QueryDsl, RunQueryDsl};

use super::schema::users;

pub(crate) const API: &str = "https://www.strava.com/api/v3";

/// check user id from the request
/// 
/// Will refresh token if possible
pub fn get_id(request: &Request) -> TbResult<i32> {
    User::get(request).map(|u| u.user.tendabike_id)
}

#[derive(Queryable, Insertable, Identifiable, Debug)]
#[table_name = "users"]
struct DbUser {
    id: i32,
    tendabike_id: i32,
    last_activity: i64,
    access_token: String,
    expires_at: i64,
    refresh_token: String,
}

impl DbUser {
    /// Get the user data from the Strava OAuth callback
    fn retrieve(conn: &AppConn, config: &Config, athlete: &serde_json::value::Value) -> TbResult<Self> {
        info!(
            "got athlete {} {}, with id {}",
            athlete["firstname"], athlete["lastname"], athlete["id"]
        );
        let strava_id = athlete["id"]
            .as_i64()
            .ok_or(OAuthError::Authorize("athlet id is no int"))? as i32;

        let user = users::table.find(strava_id).get_result::<DbUser>(conn).optional()?;
        if let Some(x) = user {
            return Ok(x);
        }

        // create user!

        let client = reqwest::blocking::Client::new();

        let user = client
            .post(&format!("{}:{}/{}", TB_URL, config.port,"user"))
            .json(athlete)
            .send().context("unable to contact backend")?
            .error_for_status().context("backend responded with error")?
            .json::<Value>().context("malformed Json")?;
        let tendabike_id = user["id"]
            .as_i64()
            .ok_or_else(|| anyhow!("athlet id is no int"))? as i32;

        let user = DbUser {
            id: strava_id,
            tendabike_id,
            access_token: "".into(),
            refresh_token: "".into(),
            expires_at: 0,
            last_activity: 0,
        };
        webhook::insert_sync(strava_id, conn)?;
        Ok(diesel::insert_into(users::table)
            .values(&user)
            .get_result(conn)?)
    }

    /// Updates the user database from a new token
    fn store(self, conn: &AppConn, cookies: &mut Cookies, token: TokenResponse<Strava>) -> TbResult<(Self, String)> {
        use schema::users::dsl::*;
        use time::*;

        let iat = get_time().sec;
        let exp = token.expires_in().unwrap() as i64 + iat - 300; // 5 Minutes buffer
        let db_user: DbUser = diesel::update(users.find(self.id))
            .set((
                access_token.eq(token.access_token()),
                expires_at.eq(exp),
                refresh_token.eq(token.refresh_token().unwrap()),
            ))
            .get_result(conn).context("Could not store user")?;

        let token = token::store(cookies, db_user.tendabike_id, iat, exp);

        Ok((db_user, token))
    }
}

pub struct User {
    user: DbUser,
    conn: AppDbConn,
    pub token: String,
    pub url: String,
}

/// User request guard
///
/// The User struct handles all authentication and provides a method to request API calls
///
/// The validity check and refresh logic is all processed in the FromRequest Trait
///
/// It retrieves the storage (cookies and database) from the request.
/// By that the visible routes only need to use the User request guard to access Strava
impl User {
    /// the get function reads the user from the cookie and other stores,
    /// if needed and possible it refreshes the access token
    fn get(request: &Request) -> TbResult<User> {
        // Get user id
        let token = token::token(request)?;
        let id = token::id(&token, token::LEEWAY)?;
        // Get the user
        let conn = request
            .guard::<AppDbConn>()
            .expect("internal db missing!!!");
        let port = request
            .guard::<State<Config>>()
            .expect("Config missing!!!").port;
        let url = format!("{}:{}", TB_URL, port);
        let user: DbUser = users::table
            .filter(users::tendabike_id.eq(id))
            .get_result(&conn.0)
            .context("user not registered")?;

        if user.expires_at > time::get_time().sec {
            return Ok(User { user, conn, token, url });
        }

        ensure!(user.expires_at != 0, Error::NotAuth("Missing Strava Authorization"));

        info!("refreshing access token");
        let auth = request
            .guard::<OAuth>()
            .expect("No oauth struct!!!");
        let tokenset = auth
            .refresh(&user.refresh_token).context("could not refresh access token")?;

        let mut cookies = request
            .guard::<Cookies>()
            .expect("Could not get Cookie store!!!");
        let (user, token) = user.store(&conn, &mut cookies, tokenset)?;

        Ok(User { user, token, conn, url })

    }
    
    /// disable a user 
    fn disable(&self, message: &'static str) -> anyhow::Error {
        use schema::users::dsl::*;

        diesel::update(users.find(self.user.id))
            .set((expires_at.eq(0), access_token.eq("")))
            .execute(&self.conn.0).context("Could not update last_activity")
            .unwrap_or_else(|err| {error!("Could not update user: {:?}", err); 0});
        anyhow!(Error::NotAuth(message))
    }

    /// request information from the Strava API
    ///
    /// will return Error::TryAgain on certain error conditions
    /// will disable the User if Strava responds with NOT_AUTH
    fn get_strava(&self, uri: &str) -> TbResult<reqwest::blocking::Response> {
        use reqwest::StatusCode;
        let resp = reqwest::blocking::Client::new()
            .get(&format!("{}{}", API, uri))
            .bearer_auth(&self.user.access_token)
            .send().context("Could not reach strava")?;

        let status = resp.status();
        if status.is_success() { return Ok(resp) }

        match status {
            StatusCode::TOO_MANY_REQUESTS | 
            StatusCode::BAD_GATEWAY | 
            StatusCode::SERVICE_UNAVAILABLE | 
            StatusCode::GATEWAY_TIMEOUT => {
                bail!(Error::TryAgain(status.canonical_reason().unwrap()))
            },
            StatusCode::UNAUTHORIZED => bail!(self.disable("Strava request authorization withdrawn")),
            _ => bail!(Error::BadRequest(
                    format!("Strava request error: {}", status.canonical_reason().unwrap_or("Unknown status received"))
                ))
        }
    }

    /// send an API call with an authenticated User
    ///
    pub fn request(&self, uri: &str) -> TbResult<String> {
        Ok(self.get_strava(uri)?
            .text().context("Could not get response body")?)
    }

    pub fn request_json(&self, uri: &str) -> TbResult<Value> {
        Ok(self.get_strava(uri)?
            .json().context("Could not parse response body")?)
    }

    pub fn tb_id(&self) -> i32 {
        self.user.tendabike_id
    }

    pub fn strava_id(&self) -> i32 {
        self.user.id
    }

    pub fn last_activity(&self) -> i64 {
        self.user.last_activity
    }

    pub fn update_last(&self, time: i64) -> TbResult<i64> {
        if self.user.last_activity >= time {
            return Ok(self.user.last_activity);
        }
        use schema::users::dsl::*;

        diesel::update(users.find(self.user.id))
            .set(last_activity.eq(time))
            .execute(&self.conn.0).context("Could not update last_activity")?;
        Ok(time)
    }

    pub fn conn(&self) -> &AppConn {
        &self.conn
    }

    pub fn logout(&self,  cookies: Cookies)  {
        token::remove(cookies);
    }
}

pub(crate) fn strava_url(who: i32, user: &User) -> TbResult<String> {
    use schema::users::dsl::*;

    let g: i32 = users
        .filter(tendabike_id.eq(who))
        .select(id)
        .first(user.conn())?;

    Ok(format!("https://strava.com/athletes/{}", &g))
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = Redirect;

    /// Get the current user
    /// Redirect to the login screen on failure
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        match User::get(request) {
            Ok(x) => Outcome::Success(x),
            Err(err) => {
                warn!("{}", err);
                Outcome::Failure((Status::Unauthorized, Redirect::to("/login")))
            }
        }
    }
}


// We need a struct Strava to identify its type
// which is needed to retrieve the request guard
#[derive(Debug)]
pub struct Strava;

#[get("/login")]
pub(crate) fn login(oauth2: OAuth2<Strava>, mut cookies: Cookies<'_>) -> TbResult<Redirect> {
    // We want the "user:read" scope. For some providers, scopes may be
    // pre-selected or restricted during application registration. We could
    // use `&[]` instead to not request any scopes, but usually scopes
    // should be requested during registation, in the redirect, or both.
    Ok(oauth2.get_redirect(&mut cookies, &["activity:read_all,profile:read_all"])?)
}

#[get("/token")]
pub(crate) fn callback(token: TokenResponse<Strava>, conn: AppDbConn, config: State<Config>, mut cookies: Cookies<'_>) -> TbResult<Redirect> {
    info!("Strava got scope {:?}", token.scope());
    let athlete = token
        .as_value()
        .get("athlete")
        .ok_or(OAuthError::Authorize("token did not include athlete"))?;

    auth::DbUser::retrieve(&conn, &config, athlete)?.store(&conn, &mut cookies, token)?;
    Ok(Redirect::to("/"))
}

pub type OAuth = OAuth2<Strava>;

pub fn fairing(config: &Config) -> impl rocket::fairing::Fairing {
    let config = OAuthConfig::from_config(config, "strava").expect("OAuth provider not configured in Rocket.toml");
    OAuth2::<Strava>::custom(
                HyperSyncRustlsAdapter::default().basic_auth(false), config)
}
