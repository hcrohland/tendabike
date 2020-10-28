
// use diesel::prelude::*;
use diesel::{self, RunQueryDsl};
use diesel::prelude::*;

use crate::*;
use schema::events;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct InEvent {
    object_type: String,
    object_id: i64,
    // Always "create," "update," or "delete."
    aspect_type: String, 	
    // hash 	For activity update events, keys can contain "title," "type," and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone). For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    updates: HashMap<String,String>,  
    // The athlete's ID.
    owner_id: i32,
    // The push subscription ID that is receiving this event.
    subscription_id: i32, 
    // The time that the event occurred.
    event_time: i64,
}
#[derive(Debug, Default, Serialize, Deserialize, Queryable, Insertable)]
pub struct Event {
    id: Option<i32>,
    pub object_type: String,
    pub object_id: i64,
    // Always "create," "update," or "delete."
    pub aspect_type: String, 	
    // hash 	For activity update events, keys can contain "title," "type," and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone). For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    updates: String,  
    // The athlete's ID.
    owner_id: i32,
    // The push subscription ID that is receiving this event.
    subscription_id: i32, 
    // The time that the event occurred.
    pub event_time: i64,
}

impl From<InEvent> for Event {
    fn from(event: InEvent) -> Self {
        Self {
            id: None,
            object_type: event.object_type,
            object_id: event.object_id,
            aspect_type: event.aspect_type,
            owner_id: event.owner_id,
            subscription_id: event.subscription_id,
            event_time: event.event_time,
            updates: serde_json::to_string(&event.updates).unwrap_or_else(|e|{ format!("{:?}", e)}),
        }
    }
}

use rocket::request::Form;
use rocket_contrib::json::Json;
use anyhow::ensure;

// compicated way to have query parameters with dots in the name
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

impl Event {
    pub fn delete (self, user: &auth::User) -> TbResult<()> {
        use schema::events::dsl::*;
        diesel::delete(events).filter(id.eq(self.id)).execute(user.conn())?;
        Ok(())
    }
}

pub fn insert_sync(owner_id: i32, conn: &AppConn) -> TbResult<()> {
    let e = Event {
        owner_id,
        object_type: "activity".to_string(),
        aspect_type: "sync".to_string(),
        ..Default::default()
    };
    diesel::insert_into(schema::events::table)
        .values(dbg!(e))
        .execute(conn)?;
    Ok(())
}

pub fn get_event(user: &auth::User) -> TbResult<Option<Event>> {
    use schema::events::dsl::*;
    Ok(events
        .filter(owner_id.eq(user.strava_id()))
        .order(event_time.asc())
        .first(user.conn())
        .optional()?
    )
}

const VERIFY_TOKEN: &str = "tendabike_strava";

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

#[get("/hooks")]
pub fn process (user: auth::User) -> ApiResult<JSummary> {
    let e = get_event(&user)?;
    if e.is_none() {
        return tbapi(Ok(JSummary::default()));
    };
    let e= e.unwrap();
    tbapi(Ok(
        match e.object_type.as_str() {
            "activity" => activity::activity_hooks(e, &user)?,
            _ => {
                warn!("skipping event {:?}", e);
                e.delete(&user)?;
                JSummary::default()
            }
        }
    ))
}

#[post("/callback", format = "json", data="<event>")]
pub fn create_event(event: Json<InEvent>, conn: AppDbConn) -> TbResult<()> {
    use schema::events::dsl::*;
    let event = event.into_inner();
    info!("received {:?}", event);
    let event: Event = event.into();
    diesel::insert_into(events).values(&event).execute(&conn.0)?;
    Ok(())
}

#[get("/callback?<hub..>")]
pub(crate) fn validate_subscription (hub: Form<Hub>) -> ApiResult<Hub> {
    let hub = hub.into_inner();
    info!("Received validation callback {:?}", hub);
    tbapi(validate(hub))
}
