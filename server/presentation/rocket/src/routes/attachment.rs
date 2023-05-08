
use super::*;
use domain::{Summary, attachment::Event};

/// route for attach API
#[post("/attach", data = "<event>")]
fn attach_rt(event: Json<Event>, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    event.into_inner().attach(user.0, &conn).map(Json)
}

/// route for detach API
#[post("/detach", data = "<event>")]
fn detach_rt(event: Json<Event>, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    event.into_inner().detach(user.0, &conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![attach_rt, detach_rt]
}
