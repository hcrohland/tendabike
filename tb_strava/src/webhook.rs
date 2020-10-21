const VERIFY_TOKEN: &str = "tendabike_strava";

struct _Event {
    object_type: String,
    object_id: i64,
    aspect_type: String, 	// Always "create," "update," or "delete."
    updates: String,  // hash 	For activity update events, keys can contain "title," "type," and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone). For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    owner_id: i32, // The athlete's ID.
    subscription_id: i32, // The push subscription ID that is receiving this event.
    event_time: i64, // The time that the event occurred.
}

use rocket::request::Form;
use tb_common::*;
use anyhow::ensure;

#[derive(FromForm, Serialize)]
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

fn validate(hub: Hub) -> TbResult<Hub> {
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

#[get("/callback?<hub..>")]
pub(crate) fn validate_subscription (hub: Form<Hub>) -> ApiResult<Hub> {
    tbapi(validate(hub.into_inner()))
}