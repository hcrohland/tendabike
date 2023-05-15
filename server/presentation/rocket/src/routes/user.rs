
use super::*;
use domain::Summary;
use ::strava::{user_summary, get_all_stats, StravaStat};


#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    let user: &User = &user;
    Json(user.clone())
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<StravaStat>> {
    get_all_stats(&conn).map(Json)
}

#[get("/summary")]
fn summary(user: strava::User, conn: AppDbConn) -> ApiResult<Summary> {
    user_summary(&user, &conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, userlist, summary]
}


