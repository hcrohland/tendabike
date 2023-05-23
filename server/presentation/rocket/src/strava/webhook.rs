use super::*;
use anyhow::ensure;
use log::trace;
use rocket::{request::{Form, FromForm}, get, post};
use rocket_contrib::json::Json;

use domain::Summary;
use ::strava::event::{InEvent, process};
use serde_derive::Serialize;

use super::User;

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
pub(crate) fn hooks (user: User, mut conn: AppDbConn) -> ApiResult<Summary> {
    user.lock(&mut conn)?;
    let res = process(&user, &mut conn);
    user.unlock(&mut conn)?;
    res.map(Json)
}

#[post("/callback", format = "json", data="<event>")]
pub(crate) fn create_event(event: Json<InEvent>, mut conn: AppDbConn) -> Result<(),ApiError> {
    let event = event.into_inner();
    trace!("Received {:#?}", event);
    event.convert()?.store(&mut conn)?;
    Ok(())
}

#[get("/callback?<hub..>")]
pub(crate) fn validate_subscription (hub: Form<Hub>) -> ApiResult<Hub> {
    let hub = hub.into_inner();
    info!("Received validation callback {:?}", hub);
    validate(hub).map(Json)
}

#[get("/sync?<time>&<user_id>")]
pub(crate) fn sync_api (time: i64, user_id: Option<i32>, _u: Admin, mut conn: AppDbConn) -> ApiResult<()> {
    let user_id = user_id.map(|u| u.into());
    ::strava::sync_users(user_id, time, &mut conn).map(Json)
}
