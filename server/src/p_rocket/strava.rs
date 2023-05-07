use rocket::http::Cookies;
use rocket::response::Redirect;
use rocket::{routes, get, post};

use super::*;
use p_rocket::jwt;
use drivers::strava::*;

pub(super) mod ui;
pub(super) mod webhook;
mod oauth;

pub use oauth::fairing;


const API: &str = "https://www.strava.com/api/v3";

/// check user id from the request
/// 
/// Will refresh token if possible
pub fn get_id(request: &Request) -> TbResult<i32> {
    StravaContext::get(request).map(|u| u.user.tb_id())
}

pub struct StravaContext {
    user: StravaUser,
    conn: AppDbConn,
}

/// User request guard
///
/// The User struct handles all authentication and provides a method to request API calls
///
/// The validity check and refresh logic is all processed in the FromRequest Trait
///
/// It retrieves the storage (cookies and database) from the request.
/// By that the visible routes only need to use the User request guard to access Strava
impl StravaContext {
    /// get the reference to the database connection
    pub fn conn(&self) -> &AppConn {
        &self.conn
    }

    /// get the reference to the StravaUser
    pub fn user(&self) -> &StravaUser {
        &self.user
    }

    /// get references to both the StravaUser and the database connection as a tuple  
    pub fn split(&self) -> (&StravaUser, &AppConn) {
        (&self.user, &self.conn)
    }

    /// the get function reads the user from the cookie and other stores,
    /// if needed and possible it refreshes the access token
    fn get(request: &Request) -> TbResult<StravaContext> {
        // Get user id
        let token = jwt::token(request)?;
        let id = jwt::id(&token)?;
        // Get the user
        let conn = request
            .guard::<AppDbConn>()
            .expect("internal db missing!!!");
        let user = StravaUser::read(id, &conn)?;

        if user.is_valid() {
            return Ok(Self {user, conn});
        }
        let user = oauth::get_user(request, user, &conn)?;
        let mut cookies = request
            .guard::<Cookies>()
            .expect("Could not get Cookie store!!!");

        jwt::store(&mut cookies, user.tendabike_id, user.expires_at);

        Ok(Self{user,conn})
    }
    
    /// disable a user 
    fn disable(&self) -> TbResult<()> {
        let (user, conn) = self.split();

        let id = user.id();
        info!("disabling user {}", id);
        event::insert_sync(id, time::get_time().sec, conn)
            .context(format!("Could insert sync for user: {:?}", id))?;
        user.update_db(conn)
    }

    /// disable a user per admin request
    ///
    /// # Errors
    ///
    /// This function will return an error if the user does not exist, is already disabled 
    /// or has open events and if strava or the database is not reachable.
    fn admin_disable(self) -> TbResult<()> {
        let (user, conn) = self.split();
    
        let (events, disabled) = user.get_stats(conn)?;

        if disabled { bail!(Error::BadRequest(String::from("user already disabled!"))) }
        if events > 0 { bail!(Error::BadRequest(String::from("user has open events!"))) }

        reqwest::blocking::Client::new()
            .post("https://www.strava.com/oauth/deauthorize")
            .query(&[("access_token" , &user.access_token)])
            .bearer_auth(&user.access_token)
            .send().context("Could not reach strava")?
            .error_for_status()?;

        warn!("User {} disabled by admin", user.tb_id());
        self.disable()
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
            StatusCode::UNAUTHORIZED => {
                self.disable()?;
                bail!(Error::NotAuth("Strava request authorization withdrawn".to_string()))
            },
            _ => bail!(Error::BadRequest(
                    format!("Strava request error: {}", status.canonical_reason().unwrap_or("Unknown status received"))
                ))
        }
    }

    /// request a Strava API call with an authenticated User
    ///
    pub fn request(&self, uri: &str) -> TbResult<String> {
        Ok(self.get_strava(uri)?
            .text().context("Could not get response body")?)
    }

    fn logout(&self,  cookies: Cookies)  {
        jwt::remove(cookies);
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for StravaContext {
    type Error = Redirect;

    /// Get the current user
    /// Redirect to the login screen on failure
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        match StravaContext::get(request) {
            Ok(x) => Outcome::Success(x),
            Err(err) => {
                warn!("{}", err);
                Outcome::Failure((Status::Unauthorized, Redirect::to("/login")))
            }
        }
    }
}