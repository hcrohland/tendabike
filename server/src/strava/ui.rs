use super::*;
use auth::User;
use rocket::response::Redirect;


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
        webhook::validate_subscription,
        webhook::create_event,
        webhook::process,
    ]
}
