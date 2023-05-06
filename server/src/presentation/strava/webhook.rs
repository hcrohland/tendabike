use rocket::request::Form;
use rocket_contrib::json::Json;

use crate::{Summary, presentation::{error::*, AppDbConn, Admin}, drivers::strava::event::*};

use super::StravaContext;


#[get("/hooks")]
pub fn hooks (context: StravaContext) -> ApiResult<Summary> {
    let (user, conn) = context.disect();
    user.lock(conn)?;
    let res = process(&context);
    user.unlock(conn)?;
    tbapi(res)
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
    tbapi(validate(hub))
}

#[get("/sync?<time>&<user_id>")]
pub fn sync_api (time: i64, user_id: Option<i32>, _u: Admin, conn: AppDbConn) -> ApiResult<()> {
    tbapi(crate::drivers::strava::sync_users(user_id, time, &conn))
}
