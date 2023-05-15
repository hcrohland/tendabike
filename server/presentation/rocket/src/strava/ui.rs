use super::*;

#[get("/bikes/<id>")]
fn redirect_gear(id: i32, conn: AppDbConn) -> Option<Redirect> {
    gear::strava_url(id, &conn).map_or_else(|_| None, |x| Some(Redirect::permanent(x)))
}

#[get("/activities/<id>")]
fn redirect_act(id: i32, conn: AppDbConn) -> Option<Redirect> {
    activity::strava_url(id, &conn).map_or_else(|_| None, |x| Some(Redirect::permanent(x)))
}

#[get("/users/<id>")]
fn redirect_user(id: i32, conn: AppDbConn) -> Option<Redirect> {
    strava_url(id, &conn.0).map_or_else(|_| None, |x| Some(Redirect::permanent(x)))
}

#[get("/logout")]
fn logout(cookies: rocket::http::Cookies) -> Redirect {
    jwt::remove(cookies);
    Redirect::to("/")
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        redirect_gear,
        redirect_act,
        redirect_user,
        logout,
        oauth::login,
        oauth::callback,
        oauth::sync,
        oauth::disable,
        webhook::validate_subscription,
        webhook::create_event,
        webhook::hooks,
        webhook::sync_api,
    ]
}