use crate::*;
use activity::*;
use auth::User;
use rocket::response::Redirect;
use serde_json::Value as jValue;


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

#[get("/next?<batch>")]
fn next(batch: Option<usize>, user: User) -> ApiResult<Vec<TbActivity>> {
    let batch = batch.unwrap_or(10);
    tbapi(
        activity::next_activities(&user, batch, None)?
            .into_iter()
            .map(|a| a.into_tb(&user))
            .collect(),
    )
}

#[get("/user")]
fn overview(user: User) -> ApiResult<jValue> {
    tbapi(user.request_json("/athlete"))
}

#[get("/logout")]
fn logout(user: User, cookies: rocket::http::Cookies) -> Redirect {
    user.logout(cookies);
    Redirect::to("/")
}

#[get("/activities")]
fn activities(user: User) -> ApiResult<jValue> {
    tbapi(user.request_json("/athlete/activities?per_page=3"))
}

#[get("/activity/<id>")]
fn activity(id: u64, user: User) -> ApiResult<jValue> {
    tbapi(user.request_json(&format!("/activities/{}", id)))
}

#[get("/gear/<id>")]
fn gear(id: String, user: User) -> ApiResult<jValue> {
    tbapi(user.request_json(&format!("/gear/{}", &id)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        // read,
        activities,
        gear,
        activity,
        overview,
        next,
        redirect_gear,
        redirect_act,
        redirect_user,
        logout,
        auth::login,
        auth::callback,
        webhook::validate_subscription,
        webhook::create_event,
        webhook::process,
    ]
}
