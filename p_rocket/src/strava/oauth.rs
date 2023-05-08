use super::*;
use anyhow::ensure;
use domain::Summary;
use rocket_oauth2::HyperSyncRustlsAdapter;
use rocket_oauth2::{OAuth2, TokenResponse, OAuthConfig};
use rocket::Config;
use log::error;

use super::MyContext;

pub fn refresh_token(user: &StravaUser, oauth: OAuth2<Strava>) -> TbResult<TokenResponse<Strava>>{
    info!("refreshing access token for strava id {}", user.id());

    ensure!(user.expires_at != 0, Error::NotAuth("User needs to authenticate".to_string()));
    
    Ok(oauth
        .refresh(&user.refresh_token).context("could not refresh access token")?)

}

// We need a struct Strava to identify its type
// which is needed to retrieve the request guard
#[derive(Debug)]
pub struct Strava;

fn process_callback(tokenset: TokenResponse<Strava>, conn: &AppConn, mut cookies: Cookies<'_>) -> TbResult<()>
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
        let user = user.update(tokenset.access_token(), tokenset.expires_in(), tokenset.refresh_token(), conn)?;
    // });
    Ok(jwt::store(&mut cookies, user.tendabike_id, user.expires_at))
}

pub type OAuth = OAuth2<Strava>;

pub fn fairing(config: &Config) -> impl rocket::fairing::Fairing {
    let config = OAuthConfig::from_config(config, "strava").expect("OAuth provider not configured in Rocket.toml");
    OAuth2::<Strava>::custom(
                HyperSyncRustlsAdapter::default().basic_auth(false), config)
}

pub fn update(user: StravaUser, tokenset: TokenResponse<Strava>, conn: &AppConn) -> TbResult<StravaUser> {
    user.update(tokenset.access_token(), tokenset.expires_in(), tokenset.refresh_token(), conn) 
}


pub fn get_user(request: &Request, user: StravaUser, conn: &AppConn) -> Result<StravaUser, anyhow::Error> {
    let oauth = request
        .guard::<OAuth>()
        .expect("No oauth struct!!!");
    let tokenset = refresh_token(&user,oauth)?;
    let user = update(user,tokenset, conn)?;
    Ok(user)
}

fn from_tb(id: i32, context: MyContext, oauth: OAuth2<Strava>) -> TbResult<MyContext> {
    let conn = context.conn;
    let user = StravaUser::read(id, &conn)?;

    if user.is_valid() {
        return Ok(MyContext {user,conn});
    }
    let tokenset = refresh_token(&user,oauth)?;
    let user = update(user, tokenset, &conn)?;
    Ok(MyContext{user,conn})
}

#[get("/login")]
pub fn login(oauth2: OAuth2<Strava>, mut cookies: Cookies<'_>) -> TbResult<Redirect> {
    // We want the "user:read" scope. For some providers, scopes may be
    // pre-selected or restricted during application registration. We could
    // use `&[]` instead to not request any scopes, but usually scopes
    // should be requested during registation, in the redirect, or both.
    Ok(oauth2.get_redirect(&mut cookies, &["activity:read_all,profile:read_all"])?)
}

#[get("/token")]
pub fn callback(token: TokenResponse<Strava>, conn: AppDbConn, cookies: Cookies<'_>) -> Result<Redirect,String> {
    match process_callback(token, &conn, cookies) {
        Err(e) => {error!("{:#?}", e); return Err(format!("{:#?}", e))},
        _ => Ok(Redirect::to("/"))
    }
}


#[get("/sync/<tbid>")]
pub fn sync(tbid: i32, _u: Admin, context: MyContext, oauth: OAuth2<Strava>) -> ApiResult<Summary> {
    let user = from_tb(tbid, context, oauth)?;
    
    super::webhook::hooks(user)
}

#[post("/disable/<tbid>")]
pub fn disable(tbid: i32, _u: Admin, context: MyContext, oauth: OAuth2<Strava>) -> ApiResult<()> {
    from_tb(tbid, context, oauth)?.admin_disable().map(rocket_contrib::json::Json)
}