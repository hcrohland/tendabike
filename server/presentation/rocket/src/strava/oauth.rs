use super::*;
use anyhow::ensure;
use domain::Summary;
use rocket_oauth2::HyperSyncRustlsAdapter;
use rocket_oauth2::{OAuth2, TokenResponse, OAuthConfig};
use rocket::Config;
use log::error;

use super::User;

pub fn refresh_token(user: &StravaUser, oauth: OAuth2<Strava>) -> AnyResult<TokenResponse<Strava>>{
    info!("refreshing access token for strava id {}", user.strava_id());

    ensure!(user.expires_at != 0, Error::NotAuth("User needs to authenticate".to_string()));
    
    Ok(oauth
        .refresh(&user.refresh_token).context("could not refresh access token")?)

}

// We need a struct Strava to identify its type
// which is needed to retrieve the request guard
#[derive(Debug)]
pub struct Strava;

fn process_callback(tokenset: TokenResponse<Strava>, conn: &AppConn, mut cookies: Cookies<'_>) -> AnyResult<()>
{
    if tokenset.scope().unwrap_or("") != "read,activity:read_all,profile:read_all" {
        bail!(Error::NotAuth(format!("Insufficient authorization {:?}", tokenset.scope())))
    }
    let athlete = tokenset
        .as_value()
        .get("athlete")
        .ok_or(Error::NotAuth("token did not include athlete".to_string()))?;

    let athlete: StravaAthlete = serde_json::from_value(athlete.clone())?;

    // conn.transaction(|| {
        let user = athlete.retrieve(conn)?;
        let user = user.update_token(tokenset.access_token(), tokenset.expires_in(), tokenset.refresh_token(), conn)?;
    // });
    Ok(jwt::store(&mut cookies, user.tendabike_id, user.expires_at))
}

pub type OAuth = OAuth2<Strava>;

pub fn fairing(config: &Config) -> impl rocket::fairing::Fairing {
    let config = OAuthConfig::from_config(config, "strava").expect("OAuth provider not configured in Rocket.toml");
    OAuth2::<Strava>::custom(
                HyperSyncRustlsAdapter::default().basic_auth(false), config)
}

pub fn update(user: StravaUser, tokenset: TokenResponse<Strava>, conn: &AppConn) -> AnyResult<StravaUser> {
    user.update_token(tokenset.access_token(), tokenset.expires_in(), tokenset.refresh_token(), conn) 
}


pub fn get_user(request: &Request, user: StravaUser, conn: &AppConn) -> Result<StravaUser, anyhow::Error> {
    let oauth = request
        .guard::<OAuth>()
        .expect("No oauth struct!!!");
    let tokenset = refresh_token(&user,oauth)?;
    let user = update(user,tokenset, conn)?;
    Ok(user)
}

fn from_tb(id: i32, oauth: OAuth2<Strava>, conn: &AppConn) -> AnyResult<StravaUser> {
    let user = StravaUser::read(id, &conn)?;

    if user.token_is_valid() {
        return Ok(user);
    }
    let tokenset = refresh_token(&user,oauth)?;
    let user = update(user, tokenset, &conn)?;
    Ok(user)
}

#[get("/login")]
pub fn login(oauth2: OAuth2<Strava>, mut cookies: Cookies<'_>) -> AnyResult<Redirect> {
    // We want the "user:read" scope. For some providers, scopes may be
    // pre-selected or restricted during application registration. We could
    // use `&[]` instead to not request any scopes, but usually scopes
    // should be requested during registation, in the redirect, or both.
    Ok(oauth2.get_redirect(&mut cookies, &["activity:read_all,profile:read_all"])?)
}

#[get("/logout")]
pub fn logout(cookies: rocket::http::Cookies) -> Redirect {
    jwt::remove(cookies);
    Redirect::to("/")
}

#[get("/token")]
pub(crate) fn callback(token: TokenResponse<Strava>, conn: AppDbConn, cookies: Cookies<'_>) -> Result<Redirect,String> {
    match process_callback(token, &conn, cookies) {
        Err(e) => {error!("{:#?}", e); return Err(format!("{:#?}", e))},
        _ => Ok(Redirect::to("/"))
    }
}


#[get("/sync/<tbid>")]
pub(crate) fn sync(tbid: i32, _u: Admin, conn: AppDbConn, oauth: OAuth2<Strava>) -> ApiResult<Summary> {
    let user = from_tb(tbid, oauth, &conn)?;
    
    super::webhook::hooks(User(user), conn)
}

#[post("/disable/<tbid>")]
pub(crate) fn disable(tbid: i32, _u: Admin, conn: AppDbConn, oauth: OAuth2<Strava>) -> ApiResult<()> {
    from_tb(tbid, oauth, &conn)?.admin_disable(&conn).map(rocket_contrib::json::Json)
}