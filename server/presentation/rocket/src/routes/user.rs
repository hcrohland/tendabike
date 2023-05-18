
use super::*;
use domain::Summary;
use ::strava::{get_all_stats, StravaStat};


#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    let user: &User = &user;
    Json(user.clone())
}

#[get("/summary")]
fn summary(user: strava::User, conn: AppDbConn) -> ApiResult<Summary> {
    user.get_summary(&conn).map(Json)
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<StravaStat>> {
    get_all_stats(&conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, summary, userlist]
}


