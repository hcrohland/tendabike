
use rocket_contrib::json::Json;

use super::*;
use domain::attachment::*;

/// route for attach API
#[post("/attach", data = "<event>")]
fn attach_rt(event: Json<Event>, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().attach(user.0, &conn))
}

/// route for detach API
#[post("/detach", data = "<event>")]
fn detach_rt(event: Json<Event>, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().detach(user.0, &conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![attach_rt, detach_rt]
}
