
use rocket_oauth2::{OAuth2, OAuthConfig, TokenResponse};
use rocket_oauth2::hyper_sync_rustls_adapter::HyperSyncRustlsAdapter;

use rocket::request::{self, Request, FromRequest};
use rocket::response::Redirect;
use rocket::*;
use rocket::http::*;

use serde_json::Value;

use crate::*;

use diesel::prelude::*;
use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

use schema::users;

const PROVIDER: rocket_oauth2::StaticProvider = rocket_oauth2::StaticProvider 
{
    auth_uri: std::borrow::Cow::Borrowed("https://www.strava.com/oauth/authorize"),
    token_uri: std::borrow::Cow::Borrowed("https://www.strava.com/oauth/token")
};

const API_URI: &str = "https://www.strava.com/api/v3";

lazy_static! {
    static ref CLIENT_ID: String = std::env::var("CLIENT_ID").expect("Couldn't read var CLIENT_ID");
    static ref CLIENT_SECRET: String = std::env::var("CLIENT_SECRET").expect("Couldn't read var CLIENT_SECRET");
}

#[derive(Queryable, Insertable, Identifiable, Debug)]
#[table_name = "users"]
struct DbUser {
    id: i32,
    tendabike_id: i32,
    last_activity: i64,
    access_token: String,
    expires_at: i64,
    refresh_token: String
}

impl DbUser {
    fn retrieve (request: &Request, athlete: &serde_json::value::Value) -> TbResult<Self> {    
        info!("got athlete {} {}, with id {}", athlete["firstname"], athlete["lastname"], athlete["id"]);
        let conn = request.guard::<StravaDbConn>().expect("internal db missing!!!").0;
        let strava_id = athlete["id"].as_i64().ok_or(StravaError::Authorize("athlet id is no int"))? as i32;

        let user = users::table.find(strava_id).get_result::<DbUser>(&conn);
        match user {
            Ok(x) => return Ok(x),
            Err(diesel::NotFound) => (),
            Err(x) => panic! (format!("database error: {}", x))
        }
        let client = reqwest::Client::new();

        let user = client.post(&format!("{}{}", TB_URI, "/user"))
            .json(athlete)
            .send().context("unable to contact backend")?
            .error_for_status().context("backend responded with error")?
            .json::<Value>().context("malformed Json")?;
        let tendabike_id = user["id"].as_i64().ok_or_else(|| anyhow!("athlet id is no int"))? as i32;

        let user = DbUser {
            id: strava_id,
            tendabike_id,
            access_token: "".into(),
            refresh_token: "".into(),
            expires_at: 0,
            last_activity: 0
        };
        Ok(diesel::insert_into(users::table).values(&user).get_result(&conn)?)
    }

    /// Updates the user data from a new token 
    fn store(self, request: &Request, token: TokenResponse) -> TbResult<Self> {
        use schema::users::dsl::*;
        use time::*;

        let conn: &AppConn = &request.guard::<StravaDbConn>().expect("No db connection");
        let iat = get_time().sec;
        let exp = token.expires_in().unwrap() as i64 + iat - 300; // 5 Minutes buffer
        let db_user: DbUser = 
            diesel::update(users.find(self.id))
                    .set((
                        access_token.eq(token.access_token()),
                        expires_at.eq(exp),
                        refresh_token.eq(token.refresh_token().unwrap())
                    ))
                    .get_result(conn).context("Could not store user")?;

        token::store(request, db_user.tendabike_id, iat, exp);
                
        Ok(db_user)
    }
}

pub struct User {
    user: DbUser,
    conn: StravaDbConn,
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
    fn get (request: &Request) -> TbResult<User> {
        // Get user id
        let token = token::token(request)?;
        let id = token::id_unsafe(&token)?;
        // Get the user
        let conn = request.guard::<StravaDbConn>().expect("internal db missing!!!");
        let user: DbUser = users::table.filter(users::tendabike_id.eq(id)).get_result(&conn.0).context("user not registered")?;

        if user.expires_at > time::get_time().sec {
            return Ok(User {user: user, conn});
        }

        info! ("refreshing access token");
        let auth = request.guard::<State<OAuth>>().expect("No oauth struct!!!");
        let tokenset = auth.refresh(&user.refresh_token).context("could not refresh access token")?;
        
        let user = user.store(request, tokenset)?;

        Ok(User{
            user,
            conn
        })
    }

    pub fn token(&self) -> &str {
        &self.user.access_token
    }

    /// send an API call with an authenticated User
    /// 
    pub fn request(&self, uri: &str) -> TbResult<String> {
        let client = reqwest::Client::new();
        Ok(client.get(&format!("{}{}", API_URI, uri))
            .bearer_auth(self.token())
            .send().context("Could not reach strava")?
            .text().context("Could not get response body")?)
    }

    pub fn request_json(&self, uri: &str) -> TbResult<Value> {
        let client = reqwest::Client::new();
        Ok(client.get(&format!("{}{}", API_URI, uri))
            .bearer_auth(self.token())
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

pub(crate) fn strava_url (who: i32, user: &User) -> TbResult<String> {
    use schema::users::dsl::*;
 
    let g: i32 = users.filter(tendabike_id.eq(who)).select(id).first(user.conn())?;
    
    Ok(format!("https://strava.com/athletes/{}", &g))
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
    type Responder = TbResult<Redirect>;
    fn callback(&self, request: &Request, token: TokenResponse)
        -> TbResult<Redirect>
    {
        info!("Callback got scope {:?}", token.scope());
        let athlete = token.as_value().get("athlete").ok_or(StravaError::Authorize("token did not include athlete"))?;
    
        DbUser::retrieve(request, athlete)?
                    .store(request, token)?;
        Ok(Redirect::to("/"))
    }
}

type OAuth = OAuth2<Callback>;

pub fn fairing () -> impl rocket::fairing::Fairing {

    let config = OAuthConfig::new(PROVIDER, CLIENT_ID.to_string(), CLIENT_SECRET.to_string(), "http://localhost:8000/token".into());

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