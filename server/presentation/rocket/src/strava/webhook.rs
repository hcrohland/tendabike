use super::*;
use anyhow::ensure;
use log::trace;
use rocket::{request::{Form, FromForm}, get, post};
use rocket_contrib::json::Json;

use domain::Summary;
use ::strava::event::{InEvent, process};
use serde_derive::Serialize;

use super::MyContext;

// complicated way to have query parameters with dots in the name
#[derive(Debug, FromForm, Serialize)]
pub struct Hub {
    #[form(field = "hub.mode")]
    #[serde(skip_serializing)]
    mode: String,
    #[form(field = "hub.challenge")]
    #[serde(rename(serialize = "hub.challenge"))]
    challenge: String,
    #[form(field = "hub.verify_token")]
    #[serde(skip_serializing)]
    verify_token: String,
}

fn validate(hub: Hub) -> AnyResult<Hub> {
    ensure!(
        hub.verify_token == VERIFY_TOKEN, 
        Error::BadRequest(format!("Unknown verify token {}", hub.verify_token))
    );
    ensure!(
        hub.mode == "subscribe", 
        Error::BadRequest(format!("Unknown mode {}", hub.mode))
    );
    Ok(hub)
}

const VERIFY_TOKEN: &str = "tendabike_strava";

#[get("/hooks")]
pub fn hooks (context: MyContext) -> ApiResult<Summary> {
    let (user, conn) = context.split();
    user.lock(conn)?;
    let res = process(user, conn);
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
    ::strava::sync_users(user_id, time, &conn).map(Json)
}
