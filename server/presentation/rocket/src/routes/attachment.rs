
use super::*;
use domain::{Summary, Event};

/// route for attach API
#[post("/attach", data = "<event>")]
fn attach_rt(event: Json<Event>, user: RUser, mut conn: AppDbConn) -> ApiResult<Summary> {
    event.into_inner().attach(&user, &mut conn).map(Json)
}

/// route for detach API
#[post("/detach", data = "<event>")]
fn detach_rt(event: Json<Event>, user: RUser, mut conn: AppDbConn) -> ApiResult<Summary> {
    event.into_inner().detach(&user, &mut conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![attach_rt, detach_rt]
}
