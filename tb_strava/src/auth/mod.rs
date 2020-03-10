use rocket_oauth2::TokenResponse;

use rocket::http::*;
use rocket::request::{self, FromRequest, Request};
use rocket::response::Redirect;
use rocket::*;

use serde_json::Value;

use crate::*;

use diesel::prelude::*;
use diesel::{self, QueryDsl, RunQueryDsl};

use schema::users;

pub mod strava;


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
    fn retrieve(request: &Request, athlete: &serde_json::value::Value) -> TbResult<Self> {
        info!(
            "got athlete {} {}, with id {}",
            athlete["firstname"], athlete["lastname"], athlete["id"]
        );
        let conn = request
            .guard::<AppDbConn>().expect("internal db missing!!!")
            .0;
        let config = request
            .guard::<State<Config>>()
            .expect("Config missing!!!");
        let strava_id = athlete["id"]
            .as_i64()
            .ok_or(OAuthError::Authorize("athlet id is no int"))? as i32;

        let user = users::table.find(strava_id).get_result::<DbUser>(&conn);
        match user {
            Ok(x) => return Ok(x),
            Err(diesel::NotFound) => (),
            Err(x) => panic!(format!("database error: {}", x)),
        }
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
        Ok(diesel::insert_into(users::table)
            .values(&user)
            .get_result(&conn)?)
    }

    /// Updates the user data from a new token
    fn store(self, request: &Request, token: TokenResponse) -> TbResult<(Self, String)> {
        use schema::users::dsl::*;
        use time::*;

        let conn: &AppConn = &request.guard::<AppDbConn>().expect("No db connection");
        let iat = get_time().sec;
        let exp = token.expires_in().unwrap() as i64 + iat - 300; // 5 Minutes buffer
        let db_user: DbUser = diesel::update(users.find(self.id))
            .set((
                access_token.eq(token.access_token()),
                expires_at.eq(exp),
                refresh_token.eq(token.refresh_token().unwrap()),
            ))
            .get_result(conn).context("Could not store user")?;

        let token = token::store(request, db_user.tendabike_id, iat, exp);

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

        info!("refreshing access token");
        let auth = request
            .guard::<State<strava::OAuth>>()
            .expect("No oauth struct!!!");
        let tokenset = auth
            .refresh(&user.refresh_token).context("could not refresh access token")?;

        let (user, token) = user.store(request, tokenset)?;

        Ok(User { user, token, conn, url })

    }
    
    /// send an API call with an authenticated User
    ///
    pub fn request(&self, uri: &str) -> TbResult<String> {
        let client = reqwest::blocking::Client::new();
        Ok(client
            .get(&format!("{}{}", strava::API, uri))
            .bearer_auth(&self.user.access_token)
            .send().context("Could not reach strava")?
            .text().context("Could not get response body")?)
    }

    pub fn request_json(&self, uri: &str) -> TbResult<Value> {
        let client = reqwest::blocking::Client::new();
        Ok(client
            .get(&format!("{}{}", strava::API, uri))
            .bearer_auth(&self.user.access_token)
            .send().context("Could not reach strava")?
            .json().context("Could not parse response body")?)
    }

    pub fn id(&self) -> i32 {
        self.user.tendabike_id
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
