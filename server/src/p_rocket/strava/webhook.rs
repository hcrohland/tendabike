use super::*;
use rocket::request::Form;
use rocket_contrib::json::Json;

use crate::drivers::strava::event::{Hub, InEvent, process, validate};

use super::StravaContext;


#[get("/hooks")]
pub fn hooks (context: StravaContext) -> ApiResult<Summary> {
    let (user, conn) = context.split();
    user.lock(conn)?;
    let res = process(&context);
    user.unlock(conn)?;
    res.map(Json)
}

#[post("/callback", format = "json", data="<event>")]
pub fn create_event(event: Json<InEvent>, conn: AppDbConn) -> Result<(),ApiError> {
    let event = event.into_inner();
    trace!("Received {:#?}", event);
    event.convert()?.store(&conn)?;
    Ok(())
}

#[get("/callback?<hub..>")]
pub fn validate_subscription (hub: Form<Hub>) -> ApiResult<Hub> {
    let hub = hub.into_inner();
    info!("Received validation callback {:?}", hub);
    validate(hub).map(Json)
}

#[get("/sync?<time>&<user_id>")]
pub fn sync_api (time: i64, user_id: Option<i32>, _u: Admin, conn: AppDbConn) -> ApiResult<()> {
    crate::drivers::strava::sync_users(user_id, time, &conn).map(Json)
}
