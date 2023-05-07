
use drivers::strava::StravaStat;
use rocket_contrib::json::Json;
use presentation::strava;

use crate::drivers::strava::{user_summary, get_all_stats};

use super::*;

#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    Json(user.0.clone())
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<StravaStat>> {
    get_all_stats(&conn).map(Json)
}

#[get("/summary")]
fn summary(context: strava::StravaContext) -> ApiResult<Summary> {
    user_summary(&context).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, userlist, summary]
}


