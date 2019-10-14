
use rocket_oauth2::{OAuth2, OAuthConfig, TokenResponse};
use rocket_oauth2::hyper_sync_rustls_adapter::HyperSyncRustlsAdapter;

use rocket::request::{self, Request, FromRequest};
use rocket::response::Redirect;
use rocket::*;
use rocket::http::*;
use crate::*;

use diesel::prelude::*;
use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

use schema::users;

const PROVIDER: rocket_oauth2::Provider = rocket_oauth2::Provider 
{
    auth_uri: std::borrow::Cow::Borrowed("https://www.strava.com/oauth/authorize"),
    token_uri: std::borrow::Cow::Borrowed("https://www.strava.com/oauth/token")
};

const API_URI: &str = "https://www.strava.com/api/v3";

lazy_static! {
    static ref CLIENT_ID: String = std::env::var("CLIENT_ID").expect("Couldn't read var CLIENT_ID");
    static ref CLIENT_SECRET: String = std::env::var("CLIENT_SECRET").expect("Couldn't read var CLIENT_SECRET");
}

#[derive(Queryable, Insertable, AsChangeset, Identifiable, Debug)]
#[table_name = "users"]
struct DbUser {
    id: i32,
    tendabike_id: Option<i32>,
    last_activity: Option<i64>,
    access_token: String,
    expires_at: i64,
    refresh_token: String
}

pub struct User{
    user: DbUser,
    conn: AppDbConn
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
    fn get (request: &Request) -> MyResult<User> {
        // Get user id
        let mut cookies = request.guard::<Cookies>().expect("request cookies");
        let id = cookies.get_private("id").ok_or(ErrorKind::Authorize("cookie id missing"))?
                               .value().parse::<i32>().chain_err(|| "parse error")?;

        // Get the user
        let conn = request.guard::<AppDbConn>().expect("internal db missing");
        let user: DbUser = users::table.find(id).get_result(&conn.0).chain_err(|| "user not registered")?;

        if user.expires_at > time::get_time().sec {
            return Ok(User {user, conn});
        }

        info! ("refreshing access token");
        let auth = request.guard::<State<OAuth>>().expect("oauth struct");
        let tokenset = auth.refresh(&user.refresh_token).map_err(|_| "could not refresh access token")?;
        
        Ok(User{
            user: User::store(request, cookies, user.id, tokenset)?,
            conn
        })
    }

    /// Updates the user data from a new token 
    /// Needs the cookies parameter to prevent two Cookie instance retrieval
    fn store(request: &Request, mut cookies: Cookies, uid: i32, token: TokenResponse) -> MyResult<DbUser> {
        use schema::users::dsl::*;
        use time::*;

        cookies.add_private(Cookie::build("id", uid.to_string()).same_site(SameSite::Lax).finish());

        let user = DbUser {
            id: uid,
            tendabike_id: None,
            last_activity: None,
            access_token: token.access_token,
            expires_at: token.expires_in.unwrap() as i64 + get_time().sec - 300, // 5 Minutes buffer
            refresh_token: token.refresh_token.unwrap()
        };
        let conn: &AppConn = &request.guard::<AppDbConn>().expect("db connection");
        let mut db_user: DbUser = 
            diesel::insert_into(users).values(&user)
                    .on_conflict(id)
                    .do_update().set(&user)
                    .get_result(conn).chain_err(|| "Could not store user")?;
        if db_user.tendabike_id.is_none() {
            db_user = diesel::update(&db_user)
                        .set(tendabike_id.eq(Some(0)))
                        .get_result(conn).chain_err(||"Could not set the remote id")?;
        }

        Ok(db_user)
    }

    /// send an API call with an authenticated User
    /// 
    pub fn request(&self, uri: &str) -> MyResult<String> {
        let client = reqwest::Client::new();
        Ok(client.get(&format!("{}{}", API_URI, uri))
            .bearer_auth(&self.user.access_token)
            .send().chain_err(|| "Could not reach strava")?
            .text().chain_err(|| "Could not get response body")?)
    }

    pub fn id(&self) -> i32 {
        self.user.tendabike_id.expect("Tendabike_id missing")
    }

    pub fn last_activity(&self) -> i64 {
        self.user.last_activity.unwrap_or(0)
    }

    pub fn update_last(&self, time: i64) -> MyResult<i64> {
        if let Some(l) = self.user.last_activity { 
            if l >= time {
                return Ok(l);
            } 
        }
        use schema::users::dsl::*;

        diesel::update(users.find(self.user.id))
                        .set(last_activity.eq(Some(time)))
                        .execute(&self.conn.0).chain_err(|| "Could not update last_activity")?;
        Ok(time)
    }

    pub fn conn(&self) -> &AppConn {
        &self.conn
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = Redirect;

    /// Get the current user
    /// Redirect to the login screen on failure
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        match User::get(request){
            Ok(x) => Outcome::Success(x),
            Err(err) => { warn!("{}", err); Outcome::Failure((Status::Unauthorized, Redirect::to("/login")))}
        }
    }
}


// We need a struct Callback to identify its type
// which is needed to retrieve the request guard
#[derive(Debug)]
struct Callback;

impl rocket_oauth2::Callback for Callback {
    type Responder = MyResult<Redirect>;
    fn callback(&self, request: &Request, token: TokenResponse)
        -> MyResult<Redirect>
    {

        info!("Callback got scope {:?}", token.scope);
        let athlete = token.extras.get("athlete").ok_or(ErrorKind::Authorize("token did not include athlete"))?;
        info!("got athlete {} {}, with id {}", athlete["firstname"], athlete["lastname"], athlete["id"]);
        let id = athlete["id"].as_i64().ok_or(ErrorKind::Authorize("athlet id is no int"))? as i32;
        let cookies = request.guard::<Cookies>().expect("request cookies");
        User::store(request, cookies, id, token)?;
        Ok(Redirect::to("/user"))
    }
}

type OAuth = OAuth2<HyperSyncRustlsAdapter, Callback>;

pub fn fairing () -> impl rocket::fairing::Fairing {

    let config = OAuthConfig::new(PROVIDER, CLIENT_ID.to_string(), CLIENT_SECRET.to_string(), "http://localhost:8001/token".into());

    // Strava uses "," instead of the standard Space as a delimter for scopes :-(
    OAuth2::custom(HyperSyncRustlsAdapter, Callback {}, config, "/token", Some(("/login", vec!["activity:read_all,profile:read_all".to_string()])))
}

#[catch(401)]
fn not_authorized(_req: &Request) -> Redirect { 
    Redirect::to("/login")
}

pub fn catchers () -> Vec<Catcher> {
    catchers![not_authorized]
}