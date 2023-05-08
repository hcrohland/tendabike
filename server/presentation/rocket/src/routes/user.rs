
use super::*;
use domain::Summary;
use domain::drivers::strava::{user_summary, get_all_stats, StravaStat};


#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    Json(user.0.clone())
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<StravaStat>> {
    get_all_stats(&conn).map(Json)
}

#[get("/summary")]
fn summary(context: strava::MyContext) -> ApiResult<Summary> {
    user_summary(&context).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, userlist, summary]
}


