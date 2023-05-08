use std::collections::HashMap;

use super::*;
use schema::strava_events;

use crate::drivers::strava::activity::StravaActivity;

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

impl InEvent {
    pub fn convert(self) -> TbResult<Event> {
        let objects = ["activity", "athlete"];
        let aspects = ["create", "update", "delete"];

        ensure!(
            objects.contains(&self.object_type.as_str()) && aspects.contains(&self.aspect_type.as_str()),
            Error::BadRequest(format!("unknown event received: {:?}", self))
        );

        Ok(Event {
            id: None,
            object_type: self.object_type,
            object_id: self.object_id,
            aspect_type: self.aspect_type,
            owner_id: self.owner_id,
            subscription_id: self.subscription_id,
            event_time: self.event_time,
            updates: serde_json::to_string(&self.updates).unwrap_or_else(|e|{ format!("{:?}", e)}),
        })
    }
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

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event {}: {} {} {} at {}, owner:{}", 
            self.id.unwrap_or(0), 
            self.aspect_type,
            self.object_type,
            self.object_id,
            self.event_time,
            self.owner_id)
    }
}

impl Event {
    fn delete (&self, conn: &AppConn) -> TbResult<()> {
        use schema::strava_events::dsl::*;
        debug!("Deleting {}", self);
        diesel::delete(strava_events).filter(id.eq(self.id)).execute(conn)?;
        Ok(())
    }

    fn setdate(&mut self, time: i64, conn: &AppConn) -> TbResult<()> {
        use schema::strava_events::dsl::*;
        self.event_time = time;
        diesel::update(strava_events).filter(id.eq(self.id)).set(event_time.eq(self.event_time)).execute(conn)?;
        Ok(())
    }

    pub fn store(self, conn: &AppConn) -> TbResult<()>{
        ensure!(
            schema::strava_users::table.find(self.owner_id).execute(conn) == Ok(1),
            Error::BadRequest(format!("Unknown event owner received: {}", self))
        );
        
        info!("Received {}", 
            diesel::insert_into(schema::strava_events::table).values(&self).get_result::<Event>(conn)?
        );
        Ok(())
    }

    fn rate_limit(self, context: &dyn StravaContext) -> TbResult<Option<Self>> {
        // rate limit event
        if self.event_time > chrono::offset::Utc::now().timestamp() {
            // still rate limited!
            return Ok(None);
        }
        // remove stop event
        warn!("Starting hooks again");
        self.delete(context.conn())?;
        // get next event
        return get_event(context)
    }

    fn process_activity (self, context: &dyn StravaContext) -> TbResult<Summary> {
        match self.process_hook(context).or_else(|e| check_try_again(e, context.conn()))    {
            Ok(res) => return Ok(res),
            Err(err) => {
                        self.delete(context.conn())?;
                        Err(err)
                    }
        }
    }
    
    fn process_hook(&self, context: &dyn StravaContext) -> TbResult<Summary>{
        let res = match self.aspect_type.as_str() {
            "create" | "update" => activity::upsert_activity(self.object_id, context)?,
            "delete" => activity::delete_activity(self.object_id, context)?,
            _ => {
                warn!("Skipping unknown aspect_type {:?}", self);
                Summary::default()
            }
        };
        self.delete(context.conn())?;
        Ok(res)
    }
    
    fn sync(mut self, context: &dyn StravaContext) -> TbResult<Summary> {
        // let mut len = batch;
        let mut start = self.event_time;
        let mut hash = SumHash::default();
    
        // while len == batch 
        {
            let acts = next_activities(context, 10, Some(start))?;
            if acts.len() == 0 {
                self.delete(context.conn())?;
            } else {
                for a in acts {
                    start = std::cmp::max(start, a.start_date.timestamp());
                    trace!("processing sync event at {}", start);
                    let ps = a.send_to_tb(context)?;
                    self.setdate(start,  context.conn())?;
                    hash.merge(ps);
                }
            }
        }
    
        Ok(hash.collect())
    }
}

pub fn insert_sync(owner_id: i32, event_time: i64, conn: &AppConn) -> TbResult<()> {
    ensure!(
        event_time <= get_time(), 
        Error::BadRequest(format!("eventtime {} > now!", event_time))
    );
    Event {
        owner_id,
        event_time,
        object_type: "sync".to_string(),
        ..Default::default()
    }.store(conn)
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

pub fn get_event(context: &dyn StravaContext) -> TbResult<Option<Event>> {
    use schema::strava_events::dsl::*;
    let (user, conn) = context.split();

    let event: Option<Event> = strava_events
        .filter(owner_id.eq_any(vec![0,user.id]))
        .order(event_time.asc())
        .first(conn)
        .optional()?;
    let event = match event {
        Some(event) => event,
        None => return Ok(None),
    };
    if event.object_type.as_str() == "stop" { 
        return event.rate_limit(context);
    }

    // Prevent unneeded calls to Strava
    // only the latest event for an object is interesting
    let mut list = strava_events
            .filter(object_id.eq(event.object_id))
            .filter(owner_id.eq(event.owner_id))
            .order(event_time.asc())
            .get_results::<Event>(conn)?;
    let res = list.pop();

    if list.len() > 0 {
        debug!("Dropping {:#?}", list);
        diesel::delete(strava_events)
        .filter(id.eq_any(
            list.into_iter().map(|l| l.id).collect::<Vec<_>>())
        )
        .execute(conn)?;
    }

    return Ok(res)
}

fn check_try_again(err: anyhow::Error, conn: &AppConn) -> TbResult<Summary> {
    // Keep events for temporary failure - delete others
    match err.downcast_ref::<Error>() {
        Some(&Error::TryAgain(_)) => {
            warn!("Stopping hooks for 15 minutes {:?}", err);
            insert_stop(conn)?;
            Ok(Summary::default())
        },
        _ => Err(err)
    }
}

fn next_activities(context: &dyn StravaContext, per_page: usize, start: Option<i64>) -> TbResult<Vec<StravaActivity>> {
    let r = context.request(&format!(
        "/activities?after={}&per_page={}",
        start.unwrap_or_else(|| context.user().last_activity),
        per_page
    ))?;
    Ok(serde_json::from_str::<Vec<StravaActivity>>(&r)?)
}

pub fn process (context: &dyn StravaContext) -> TbResult<Summary> {
    let event = get_event(context)?;
    if event.is_none() {
        return Ok(Summary::default());
    };
    let event = event.unwrap();
    info!("Processing {}", event);
    
    match event.object_type.as_str() {
        "activity" => event.process_activity(context),
        "sync" => event.sync(context).or_else(|err| check_try_again(err, context.conn())),
        // "athlete" => process_user(e, user),
        _ => {
            warn!("skipping {}", event);
            event.delete(context.conn())?;
            Ok(Summary::default())
        }
    }
}