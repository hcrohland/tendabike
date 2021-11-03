use super::*;
use auth::User;
use rocket::response::Redirect;


/// Get list of gear for user from Strava
pub fn update_user(user: &User) -> TbResult<Vec<PartId>> {
    #[derive(Deserialize, Debug)]
    struct Gear {
        id: String,
    }

    #[derive(Deserialize, Debug)]
    struct Athlete {
        // firstname: String,
        // lastname: String,
        bikes: Vec<Gear>,
        shoes: Vec<Gear>,
    }

    let r = user.request("/athlete")?;
    let ath: Athlete = serde_json::from_str(&r)?;
    let parts = ath.bikes.into_iter()
        .chain(ath.shoes)
        .map(|gear| gear::strava_to_tb(gear.id, user))
        .collect::<TbResult<_>>()?;
    Ok(parts)
}

#[get("/bikes/<id>")]
fn redirect_gear(id: i32, user: User) -> Option<Redirect> {
    gear::strava_url(id, &user).map_or_else(|_| None, |x| Some(Redirect::permanent(x)))
}

#[get("/activities/<id>")]
fn redirect_act(id: i32, user: User) -> Option<Redirect> {
    activity::strava_url(id, &user).map_or_else(|_| None, |x| Some(Redirect::permanent(x)))
}

#[get("/users/<id>")]
fn redirect_user(id: i32, user: User) -> Option<Redirect> {
    auth::strava_url(id, &user).map_or_else(|_| None, |x| Some(Redirect::permanent(x)))
}

#[get("/logout")]
fn logout(user: User, cookies: rocket::http::Cookies) -> Redirect {
    user.logout(cookies);
    Redirect::to("/")
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        redirect_gear,
        redirect_act,
        redirect_user,
        logout,
        auth::login,
        auth::callback,
        auth::sync,
        auth::disable,
        webhook::validate_subscription,
        webhook::create_event,
        webhook::hooks,
        webhook::sync_api,
    ]
}
