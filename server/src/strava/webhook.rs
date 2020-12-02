use std::collections::HashMap;
use std::convert::TryInto;

use rocket::request::Form;
use rocket_contrib::json::Json;
use anyhow::ensure;

use diesel::{self, RunQueryDsl};
use diesel::prelude::*;

use super::*;
use schema::strava_events;

#[derive(Debug, Serialize, Deserialize)]
pub struct InEvent {
    object_type: String,
    object_id: i64,
    // Always "create," "update," or "delete."
    aspect_type: String, 	
    // hash 	For activity update strava_events, keys can contain "title," "type," and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone). For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    updates: HashMap<String,String>,  
    // The athlete's ID.
    owner_id: i32,
    // The push subscription ID that is receiving this event.
    subscription_id: i32, 
    // The time that the event occurred.
    event_time: i64,
}
#[derive(Debug, Default, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "strava_events"]
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

impl std::convert::TryFrom<InEvent> for Event {
    type Error = anyhow::Error;

    fn try_from(event: InEvent) -> Result<Self, Self::Error> {
        let objects = ["activity", "athlete"];
        let aspects = ["create", "update", "delete"];

        ensure!(
            objects.contains(&event.object_type.as_str()) && aspects.contains(&event.aspect_type.as_str()),
            Error::BadRequest(format!("unknown event received: {:?}", event))
        );

        Ok(Self {
            id: None,
            object_type: event.object_type,
            object_id: event.object_id,
            aspect_type: event.aspect_type,
            owner_id: event.owner_id,
            subscription_id: event.subscription_id,
            event_time: event.event_time,
            updates: serde_json::to_string(&event.updates).unwrap_or_else(|e|{ format!("{:?}", e)}),
        })
    }
}

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

impl Event {
    pub fn delete (&self, conn: &AppConn) -> TbResult<()> {
        use schema::strava_events::dsl::*;
        diesel::delete(strava_events).filter(id.eq(self.id)).execute(conn)?;
        Ok(())
    }
}

fn store_event(event: Event, conn: &AppConn) -> TbResult<()>{
    ensure!(
        schema::strava_users::table.find(event.owner_id).execute(conn) == Ok(1),
        Error::BadRequest(format!("unknown event received: {:?}", event))
    );
    
    diesel::insert_into(schema::strava_events::table).values(&event).execute(conn)?;
    Ok(())
}

pub fn insert_sync(owner_id: i32, conn: &AppConn) -> TbResult<()> {
    let e = Event {
        owner_id,
        object_type: "activity".to_string(),
        aspect_type: "sync".to_string(),
        event_time: 10,
        ..Default::default()
    };
    diesel::insert_into(schema::strava_events::table)
        .values(e)
        .execute(conn)?;
    Ok(())
}

pub fn insert_stop(conn: &AppConn) -> TbResult<()> {
    let e = Event {
        object_type: "stop".to_string(),
        object_id: chrono::offset::Utc::now().timestamp() + 900,
        ..Default::default()
    };
    diesel::insert_into(schema::strava_events::table)
        .values(e)
        .execute(conn)?;
    Ok(())
}

fn rate_limit(event: Event, user: &auth::User) -> TbResult<Option<Event>> {
    // rate limit event
    if event.event_time > chrono::offset::Utc::now().timestamp() {
        // still rate limited!
        return Ok(None);
    }
    // remove stop event
    warn!("Starting hooks again");
    event.delete(user.conn())?;
    // get next event
    return get_event(user)
}

pub fn get_event(user: &auth::User) -> TbResult<Option<Event>> {
    use schema::strava_events::dsl::*;
    let conn = user.conn();

    let event: Option<Event> = strava_events
        .filter(owner_id.eq_any(vec![0,user.strava_id()]))
        .order(event_time.asc())
        .first(conn)
        .optional()?;
    let event = match event {
        Some(event) => event,
        None => return Ok(None),
    };
    if event.object_type.as_str() == "stop" { 
        return rate_limit(event, user);
    }

    // Prevent unneeded calls to Strava
    // only the latest event for an object is interesting
    let mut list = strava_events
            .filter(object_id.eq(event.object_id))
            .order(event_time.asc())
            .get_results::<Event>(conn)?;
    let res = list.pop();

    if list.len() > 0 {
        info!("dropping {:#?}", list);
        diesel::delete(strava_events)
        .filter(id.eq_any(
            list.into_iter().map(|l| l.id).collect::<Vec<_>>())
        )
        .execute(conn)?;
    }

    return Ok(res)
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
pub fn process (user: auth::User) -> ApiResult<Summary> {
    let e = get_event(&user)?;
    if e.is_none() {
        return tbapi(Ok(Summary::default()));
    };
    let e = e.unwrap();

    tbapi(match e.object_type.as_str() {
        "activity" => process_activity(e, user),
        // "athlete" => process_user(e, user),
        _ => {
            warn!("skipping {:#?}", e);
            e.delete(user.conn())?;
            Ok(Summary::default())
        }
    })
}

fn process_activity (e:Event, user: auth::User) -> TbResult<Summary> {
    info!("Processing {:#?}", e);
    
    match activity::process_hook(&e, &user) {
        Ok(res) => return Ok(res),
        Err(err) => {
            // Keep events for temporary failure - delete others
            match err.downcast_ref::<Error>() {
                Some(&Error::TryAgain(_)) => {
                    warn!("stopping hooks for 15 minutes {:?}", err);
                    webhook::insert_stop(user.conn())?;
                    Ok(Summary::default())
                },
                _ => {
                    e.delete(user.conn())?;
                    Err(err)
                }
            }
        }
    }
}

#[post("/callback", format = "json", data="<event>")]
pub fn create_event(event: Json<InEvent>, conn: AppDbConn) -> Result<(),ApiError> {
    let event = event.into_inner();
    info!("received {:#?}", event);
    store_event(event.try_into()?, &conn)?;
    Ok(())
}

#[get("/callback?<hub..>")]
pub(crate) fn validate_subscription (hub: Form<Hub>) -> ApiResult<Hub> {
    let hub = hub.into_inner();
    info!("Received validation callback {:?}", hub);
    tbapi(validate(hub))
}
