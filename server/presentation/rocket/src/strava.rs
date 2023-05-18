use anyhow::{Context, bail};
use rocket::http::Cookies;
use rocket::response::Redirect;
use rocket::{routes, get, post};

use s_diesel::AppConn;
use ::strava::*;

use super::*;
use crate::jwt;

pub(super) mod ui;
pub(super) mod webhook;
mod oauth;

pub use oauth::fairing;


/// check user id from the request
/// 
/// Will refresh token if possible
pub fn get_id(request: &Request) -> AnyResult<i32> {
    User::get(request).map(|u| u.tb_id())
}

pub struct User(StravaUser);

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
    fn get(request: &Request) -> AnyResult<User> {
        // Get user id
        let token = jwt::token(request)?;
        let id = jwt::id(&token)?;
        // Get the user
        let mut conn = request
            .guard::<AppDbConn>()
            .expect("internal db missing!!!");
        let user = StravaUser::read(id, &mut conn)?;

        if user.token_is_valid() {
            return Ok(Self(user));
        }
        let user = oauth::get_user(request, user, &mut conn)?;
        let mut cookies = request
            .guard::<Cookies>()
            .expect("Could not get Cookie store!!!");

        jwt::store(&mut cookies, user.tendabike_id, user.expires_at);

        Ok(Self(user))
    }   
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

impl Deref for User {
    type Target = StravaUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}