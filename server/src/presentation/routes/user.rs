
use domain::user::Stat;
use rocket_contrib::json::Json;
use presentation::strava;

use crate::drivers::strava::user_summary;

use super::*;

#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    Json(user.0.clone())
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<Stat>> {
    User::get_all(&conn).map(Json)
}

#[get("/summary")]
fn summary(context: strava::StravaContext) -> ApiResult<Summary> {
    user_summary(&context).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, userlist, summary]
}


