
use super::*;
use domain::Summary;
use ::strava::{get_all_stats, StravaStat};


#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    let user: &User = &user;
    Json(user.clone())
}

#[get("/summary")]
fn summary(user: strava::User, mut conn: AppDbConn) -> ApiResult<Summary> {
    user.get_summary(&mut conn).map(Json)
}

#[get("/all")]
fn userlist(_u: Admin, mut conn: AppDbConn) -> ApiResult<Vec<StravaStat>> {
    get_all_stats(&mut conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, summary, userlist]
}


